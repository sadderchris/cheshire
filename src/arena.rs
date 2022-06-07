use gc_arena::make_arena;

use super::vm::VirtualMachine;

make_arena!(pub GcArena, VirtualMachine);
