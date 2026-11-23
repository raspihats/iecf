(* CODESYS's and TwinCAT's structures holding blocks set up by their
   FB_init (struct-blocks), as TcOpen's TcoStruct: an ABSTRACT base
   structure whose member Self is given the structure itself (THIS^),
   a structure extending it whose members are given Self, a sibling.
   Worked by hand, h's d set up with Parent := THIS^ (h) and tag := 5,
   then each member's FB_init in the order declared:
   Self.FB_init(s := d): Self references d.
   a.FB_init(p := Self, k := 1): up := Self.Id(), d.tag + 100 = 105.
   b.FB_init(p := Self, k := 2): n := 2.
   %QW0 d.a.up 105, %QW2 d.b.n 2, %QW4 Self.ParentId(), d.Parent.Id(),
   h's 42, %QW6 Self.Id() 105.
   d.inner, an Inner (a Base too) given Parent := Self, d's own member,
   and tag 9: its c.FB_init(p := inner.Self): 9 + 100, %QW8 109; its
   Self.ParentId() is its Parent's Id, d.Self's, %QW10 105.
   reference-skip: not IEC 61131-3. *)
INTERFACE IObj
  METHOD Id : INT
  END_METHOD
END_INTERFACE

TYPE
  ABSTRACT Base : STRUCT
    Parent : IObj;
    Self : Marker(THIS^);
    tag : INT := 7;
  END_STRUCT;
END_TYPE

TYPE Inner EXTENDS Base :
STRUCT
  c : Leaf(Self, 3);
END_STRUCT
END_TYPE

TYPE Data EXTENDS Base :
STRUCT
  a : Leaf(Self, 1);
  b : Leaf(Self, 2);
  inner : Inner := (Parent := Self, tag := 9);
END_STRUCT
END_TYPE

FUNCTION_BLOCK Marker IMPLEMENTS IObj
  VAR st : REFERENCE TO Base; END_VAR
  METHOD FB_init : BOOL
    VAR_INPUT bInitRetains : BOOL; bInCopyCode : BOOL; s : REFERENCE TO Base; END_VAR
    st REF= s;
  END_METHOD
  METHOD PUBLIC Id : INT
    Id := st.tag + 100;
  END_METHOD
  METHOD PUBLIC ParentId : INT
    ParentId := st.Parent.Id();
  END_METHOD
END_FUNCTION_BLOCK

FUNCTION_BLOCK Leaf
  VAR_OUTPUT up : INT; n : INT; END_VAR
  METHOD FB_init : BOOL
    VAR_INPUT bInitRetains : BOOL; bInCopyCode : BOOL; p : IObj; k : INT; END_VAR
    up := p.Id();
    n := k;
  END_METHOD
END_FUNCTION_BLOCK

FUNCTION_BLOCK Holder IMPLEMENTS IObj
  VAR d : Data := (Parent := THIS^, tag := 5); END_VAR
  VAR_OUTPUT o1 : INT; o2 : INT; o3 : INT; o4 : INT; o5 : INT; o6 : INT; END_VAR
  METHOD PUBLIC Id : INT
    Id := 42;
  END_METHOD
  o1 := d.a.up;
  o2 := d.b.n;
  o3 := d.Self.ParentId();
  o4 := d.Self.Id();
  o5 := d.inner.c.up;
  o6 := d.inner.Self.ParentId();
END_FUNCTION_BLOCK

PROGRAM main
  VAR h : Holder; END_VAR
  VAR q0 AT %QW0 : INT; q1 AT %QW2 : INT; q2 AT %QW4 : INT; q3 AT %QW6 : INT; q4 AT %QW8 : INT;
      q5 AT %QW10 : INT; END_VAR
  h();
  q0 := h.o1; q1 := h.o2; q2 := h.o3; q3 := h.o4; q4 := h.o5; q5 := h.o6;
END_PROGRAM

CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION
