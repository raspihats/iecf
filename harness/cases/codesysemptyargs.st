(* CODESYS's and TwinCAT's empty arguments (empty-arguments): a
   parameter named and given nothing, `x :=,` or `y =>,`, as their
   input assistant writes every one; TcOpen's EtherCAT calls do. It is
   left out: an instance's input keeps what it had, an output is not
   read, a FUNCTION's input is its initial value.
   Worked by hand: c.n grows by inc each call, en TRUE.
   tick 0: c(inc := 2, en :=,...): n 2, x 2; c(inc :=, ...): inc
   still 2, n 4, y 4; add3(a := 1, b :=): 1 + 10 = 11.
   A last `,` before `)` is one with no name, as TcOpen's WpfContext
   writes `outHopper => ..., );`: d(inc := 3, n => v, ) gives v 3,
   then 6; add3(a := 2, ) 12.
   tick 1: x 6, y 8, z 11.
   reference-skip: not IEC 61131-3. *)
FUNCTION_BLOCK Cnt
  VAR_INPUT inc : INT := 1; en : BOOL := TRUE; END_VAR
  VAR_OUTPUT n : INT; busy : BOOL; END_VAR
  IF en THEN n := n + inc; END_IF;
  busy := n > 5;
END_FUNCTION_BLOCK

FUNCTION add3 : INT
  VAR_INPUT a : INT; b : INT := 10; END_VAR
  add3 := a + b;
END_FUNCTION

PROGRAM main
  VAR c : Cnt; d : Cnt; END_VAR
  VAR x AT %QB0 : SINT; y AT %QB1 : SINT; z AT %QB2 : SINT; v AT %QB3 : SINT; w AT %QB4 : SINT; END_VAR
  c(inc := 2, en :=, busy =>, n => x);
  c(inc :=,
    en :=,
    busy =>,
    n => y);
  z := TO_SINT(add3(a := 1, b :=));
  d(inc := 3,
    n => v,
   );
  w := TO_SINT(add3(a := 2, ));
END_PROGRAM

CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION
