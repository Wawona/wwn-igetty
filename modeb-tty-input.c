/* Mach inputd subscribe. Bitfield port descriptors stay in C. */
#include <mach/mach.h>
#include <mach/message.h>
#include <mach/task.h>
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

/*
 * Mirror iland drm.c: after Classic unloads WindowServer the client
 * bootstrap is a session subset. look_up of system MachServices fails;
 * walk bootstrap_parent then retarget TASK_BOOTSTRAP_PORT.
 */
static int input_bootstrap_look_up(const char *name, mach_port_t *out) {
  mach_port_t bp = bootstrap_port;
  mach_port_t root = bootstrap_port;
  kern_return_t kr = bootstrap_look_up(bp, (char *)name, out);
  if (kr == KERN_SUCCESS)
    return 0;

  fprintf(stderr, "[igettyd] inputd look_up %s: kr=%d %s (trying parents)\n",
          name, (int)kr, mach_error_string(kr));

  for (int depth = 1; depth <= 8; depth++) {
    mach_port_t parent = MACH_PORT_NULL;
    kern_return_t pkr = bootstrap_parent(bp, &parent);
    if (pkr != KERN_SUCCESS || parent == MACH_PORT_NULL || parent == bp) {
      fprintf(stderr, "[igettyd] bootstrap_parent stop depth=%d pkr=%d\n",
              depth, (int)pkr);
      break;
    }
    if (bp != bootstrap_port && bp != root)
      mach_port_deallocate(mach_task_self(), bp);
    bp = parent;
    root = parent;
    kr = bootstrap_look_up(bp, (char *)name, out);
    fprintf(stderr, "[igettyd] look_up via parent depth=%d kr=%d %s\n", depth,
            (int)kr, mach_error_string(kr));
    if (kr == KERN_SUCCESS) {
      if (bp != bootstrap_port)
        mach_port_deallocate(mach_task_self(), bp);
      return 0;
    }
  }

  if (root != MACH_PORT_NULL && root != bootstrap_port) {
    kern_return_t skr =
        task_set_special_port(mach_task_self(), TASK_BOOTSTRAP_PORT, root);
    fprintf(stderr, "[igettyd] task_set_bootstrap_port root kr=%d\n",
            (int)skr);
    if (skr == KERN_SUCCESS) {
      bootstrap_port = root;
      kr = bootstrap_look_up(bootstrap_port, (char *)name, out);
      fprintf(stderr, "[igettyd] look_up after retarget kr=%d %s\n", (int)kr,
              mach_error_string(kr));
      if (kr == KERN_SUCCESS)
        return 0;
    }
  }

  if (bp != bootstrap_port && bp != root)
    mach_port_deallocate(mach_task_self(), bp);
  fprintf(stderr, "[igettyd] inputd look_up %s failed: %s\n", name,
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
