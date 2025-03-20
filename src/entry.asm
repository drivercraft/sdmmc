.section .text.entry
    .globl _start
_start:
    ldr x30, =boot_stack_top
    mov sp, x30
    bl rust_main

    .section .bss.stack
    .globl boot_stack_lower_bound
boot_stack_lower_bound:
    .space 4096 * 16
    .globl boot_stack_top
boot_stack_top: