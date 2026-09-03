#ifndef WWN_IGETTY_IOS_H
#define WWN_IGETTY_IOS_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

enum wwn_igetty_session_kind {
  WWN_IGETTY_SESSION_GREETER = 0,
  WWN_IGETTY_SESSION_TEXT = 1,
  WWN_IGETTY_SESSION_NATIVE = 2,
  WWN_IGETTY_SESSION_VM = 3,
  WWN_IGETTY_SESSION_CONTAINER = 4,
  WWN_IGETTY_SESSION_COMPOSITOR = 5,
};

typedef int32_t (*wwn_igetty_present_session_fn)(void *context,
                                                 uint32_t session_id,
                                                 uint8_t kind,
                                                 const char *label);

struct wwn_igetty_ios_callbacks {
  void *context;
  wwn_igetty_present_session_fn present_session;
};

int32_t
wwn_igetty_ios_initialize(struct wwn_igetty_ios_callbacks callbacks);
uint32_t wwn_igetty_ios_register_session(uint8_t kind, const char *label);
uint32_t wwn_igetty_ios_spawn_text_session(const char *shell_path,
                                           const char *label, uint16_t rows,
                                           uint16_t cols);
uint32_t wwn_igetty_ios_adopt_live_text_sessions(void);
int32_t wwn_igetty_ios_switch_to(uint32_t session_id);
void wwn_igetty_ios_unregister_session(uint32_t session_id);
int32_t wwn_igetty_ios_session_master(uint32_t session_id);
uint32_t wwn_igetty_ios_active_session(void);
size_t wwn_igetty_ios_session_count(void);
int32_t wwn_igetty_ios_session_at(size_t index, uint32_t *out_id,
                                  uint8_t *out_kind, char *label,
                                  size_t label_capacity);
void wwn_igetty_ios_shutdown(void);

#ifdef __cplusplus
}
#endif

#endif
