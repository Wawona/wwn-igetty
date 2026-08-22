/* Mach inputd subscribe. Bitfield port descriptors stay in C. */
#include <mach/mach.h>
#include <mach/message.h>
#include <bootstrap.h>
#include <stdio.h>
#include <string.h>
#include "input_ipc.h"

#ifndef MODEB_IPC_TRAILER
#define MODEB_IPC_TRAILER 256
#endif

extern int modeb_rs_should_run(void);
extern void modeb_rs_handle_key(int key, int pressed);

static mach_port_t g_input_recv = MACH_PORT_NULL;

static int input_bootstrap_look_up(const char *name, mach_port_t *out) {
  mach_port_t bp = MACH_PORT_NULL;
  task_get_bootstrap_port(mach_task_self(), &bp);
  kern_return_t kr = bootstrap_look_up(bp, (char *)name, out);
  if (kr == KERN_SUCCESS)
    return 0;
  for (int depth = 0; depth < 8; depth++) {
    mach_port_t parent = MACH_PORT_NULL;
    kern_return_t pkr = bootstrap_parent(bp, &parent);
    if (pkr != KERN_SUCCESS || parent == MACH_PORT_NULL || parent == bp)
      break;
    if (bp != bootstrap_port)
      mach_port_deallocate(mach_task_self(), bp);
    bp = parent;
    kr = bootstrap_look_up(bp, (char *)name, out);
    if (kr == KERN_SUCCESS) {
      if (bp != bootstrap_port)
        mach_port_deallocate(mach_task_self(), bp);
      return 0;
    }
  }
  if (bp != bootstrap_port)
    mach_port_deallocate(mach_task_self(), bp);
  fprintf(stderr, "[igettyd] inputd look_up %s: %s\n", name,
          mach_error_string(kr));
  return -1;
}

int modeb_input_subscribe(void) {
  mach_port_t service = MACH_PORT_NULL;
  if (input_bootstrap_look_up(INPUT_IPC_SERVICE_NAME, &service) != 0)
    return -1;
  kern_return_t kr = mach_port_allocate(mach_task_self(), MACH_PORT_RIGHT_RECEIVE,
                                        &g_input_recv);
  if (kr != KERN_SUCCESS)
    return -1;

  input_ipc_subscribe_t sub;
  memset(&sub, 0, sizeof(sub));
  sub.header.msgh_bits =
      MACH_MSGH_BITS_COMPLEX | MACH_MSGH_BITS(MACH_MSG_TYPE_COPY_SEND, 0);
  sub.header.msgh_remote_port = service;
  sub.header.msgh_local_port = MACH_PORT_NULL;
  sub.header.msgh_id = INPUT_IPC_SUBSCRIBE_ID;
  sub.header.msgh_size = sizeof(sub);
  sub.body.msgh_descriptor_count = 1;
  sub.client_port.name = g_input_recv;
  sub.client_port.disposition = MACH_MSG_TYPE_MAKE_SEND;
  sub.client_port.type = MACH_MSG_PORT_DESCRIPTOR;
  kr = mach_msg(&sub.header, MACH_SEND_MSG, sizeof(sub), 0, MACH_PORT_NULL,
                MACH_MSG_TIMEOUT_NONE, MACH_PORT_NULL);
  if (kr != KERN_SUCCESS) {
    fprintf(stderr, "[igettyd] subscribe: %s\n", mach_error_string(kr));
    return -1;
  }
  fprintf(stderr, "[igettyd] subscribed to inputd\n");
  return 0;
}

void *modeb_input_thread(void *arg) {
  (void)arg;
  while (modeb_rs_should_run()) {
    struct {
      input_ipc_event_t ev;
      uint8_t trailer[MODEB_IPC_TRAILER];
    } buf;
    memset(&buf, 0, sizeof(buf));
    kern_return_t kr =
        mach_msg(&buf.ev.header, MACH_RCV_MSG | MACH_RCV_TIMEOUT, 0,
                 sizeof(buf), g_input_recv, 100, MACH_PORT_NULL);
    if (kr == MACH_RCV_TIMED_OUT)
      continue;
    if (kr != KERN_SUCCESS) {
      fprintf(stderr, "[igettyd] mach_msg recv: kr=0x%x %s\n", (unsigned)kr,
              mach_error_string(kr));
      continue;
    }
    if (buf.ev.event_type == INPUT_IPC_EVENT_KEYBOARD_KEY)
      modeb_rs_handle_key(buf.ev.key, buf.ev.key_state == 1);
  }
  return NULL;
}
