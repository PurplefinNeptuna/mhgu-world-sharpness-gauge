/* Selected ARM32 wrappers from the pinned libnx svc32.s.
 * Copyright (c) 2019 Hydrosphere Developers, MIT license; see THIRD_PARTY.md.
 * Private names avoid resolving unavailable Forge exports or libnx state.
 */
.syntax unified
.arm
.macro DN_SVC_ENTRY name
 .section .text.\name,"ax",%progbits
 .global \name
 .type \name,%function
 .align 2
\name:
.endm
DN_SVC_ENTRY wsg_svc_query
 str r1, [sp, #-4]!
 svc 0x6
 ldr r2, [sp]
 str r1, [r2]
 add sp, sp, #4
 bx lr
DN_SVC_ENTRY wsg_svc_get_info
 str r0, [sp, #-4]!
 ldr r0, [sp, #4]
 ldr r3, [sp, #8]
 svc 0x29
 ldr r3, [sp]
 str r1, [r3]
 str r2, [r3, #4]
 add sp, sp, #4
 bx lr
DN_SVC_ENTRY wsg_svc_flush
 str r4, [sp, #-4]!
 ldr r1, [sp, #4]
 ldr r4, [sp, #8]
 svc 0x5f
 pop {r4}
 bx lr
.section .note.GNU-stack,"",%progbits
