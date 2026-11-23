(* Programs as single instances (the program-instances extension), as
   CODESYS and TwinCAT have them. counter runs in its task with inc 1
   and total to sum, and main, after it, calls it with inc 100, calls
   its method Bump(1) and adds 10 to its calls: one instance, shared.
   Worked by hand: tick 0 - the task: total 1, calls 1, sum 1; main:
   total 101, calls 2, Bump 102, calls 12. Tick 1: total 103, calls 13,
   sum 103; 203, 14, 204, 24. Tick 2: 205, 25, 205; 305, 26, 306, 36.
   So %QB1 (calls) 12 24 36, %QB2 (Bump MOD 100) 2 4 6, %QB3 (sum MOD
   100) 1 3 5. NO.v is NO's v, not a value of E1 or E2: h's FB_init
   took it, 6, as h was set up (%QB0); main adds 1 each tick, %QB4 7 8
   9; e keeps E2#No (%QB5 1).
   reference-skip: a program read and called by its name is CODESYS's,
   not IEC 61131-3. *)
{attribute 'qualified_only'}
TYPE E1 : (No := 1, Yes := 2); END_TYPE
{attribute 'qualified_only'}
TYPE E2 : (No := 3, Maybe := 4); END_TYPE

PROGRAM NO
VAR v : INT := 6; END_VAR
END_PROGRAM

PROGRAM counter
VAR_INPUT inc : INT; END_VAR
VAR_OUTPUT total : INT; END_VAR
VAR calls : INT; END_VAR
METHOD Bump : INT
VAR_INPUT n : INT; END_VAR
total := total + n;
Bump := total;
END_METHOD
total := total + inc;
calls := calls + 1;
END_PROGRAM

FUNCTION_BLOCK Holder
VAR_OUTPUT got : INT; END_VAR
METHOD FB_init : BOOL
VAR_INPUT bInitRetains : BOOL; bInCopyCode : BOOL; start : INT; END_VAR
got := start;
END_METHOD
END_FUNCTION_BLOCK

PROGRAM main
VAR h : Holder(NO.v); r : INT; e : E2 := E2.No; END_VAR
VAR_EXTERNAL sum : INT; END_VAR
VAR q0 AT %QB0 : SINT; q1 AT %QB1 : SINT; q2 AT %QB2 : SINT; q3 AT %QB3 : SINT; q4 AT %QB4 : SINT;
    q5 AT %QB5 : SINT; END_VAR
counter(inc := 100);
r := counter.Bump(1);
counter.calls := counter.calls + 10;
NO.v := NO.v + 1;
q0 := INT_TO_SINT(h.got);
q1 := INT_TO_SINT(counter.calls);
q2 := INT_TO_SINT(r MOD 100);
q3 := INT_TO_SINT(sum MOD 100);
q4 := INT_TO_SINT(NO.v);
q5 := BOOL_TO_SINT(e = E2.No);
END_PROGRAM

CONFIGURATION cf
  VAR_GLOBAL sum : INT; END_VAR
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM c WITH tk : counter(inc := 1, total => sum);
    PROGRAM m WITH tk : main;
  END_RESOURCE
END_CONFIGURATION
