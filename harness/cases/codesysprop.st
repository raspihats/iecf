(* CODESYS's PROPERTY (properties), as TcOpen writes them. Worked by hand:
   %QW0: m.Speed := 150, the SET clamps it to 100; read back: 100.
   %QW2: m.Speed := 40, then Boost, inside Motor, Speed := Speed + 5
   through the property itself: 45.
   %QW4: Sets, read only, counts the SET's three calls: 3.
   %QW6: Level through the interface IHasLevel, Speed / 10: 4.
   %QW8: a STRING property: Name := 'xy', its GET adds '!', Twice reads
   THIS^.Name: LEN('xy!') * 2 = 6 (with codesys-oo).
   %QW10, %QW12: Status, a REFERENCE TO a structure (with reference-to),
   written through: Run twice adds 1 to state, 2; d.Status.n := 7, 7.
   reference-skip: not IEC 61131-3. *)
INTERFACE IHasLevel
  PROPERTY Level : INT
    GET
    END_GET
  END_PROPERTY
END_INTERFACE

FUNCTION_BLOCK Motor IMPLEMENTS IHasLevel
  VAR _speed : INT; _sets : INT; END_VAR
  PROPERTY PUBLIC Speed : INT
    GET
      Speed := _speed;
    END_GET
    SET
      _sets := _sets + 1;
      IF Speed > 100 THEN
        _speed := 100;
      ELSE
        _speed := Speed;
      END_IF;
    END_SET
  END_PROPERTY
  PROPERTY PUBLIC Sets : INT
    GET
      Sets := _sets;
    END_GET
  END_PROPERTY
  PROPERTY PUBLIC Level : INT
    GET
      Level := Speed / 10;
    END_GET
  END_PROPERTY
  METHOD PUBLIC Boost
    Speed := Speed + 5;
  END_METHOD
END_FUNCTION_BLOCK

FUNCTION_BLOCK Named
  VAR _n : STRING := 'abc'; END_VAR
  PROPERTY PUBLIC Name : STRING
    GET
      Name := CONCAT(_n, '!');
    END_GET
    SET
      _n := Name;
    END_SET
  END_PROPERTY
  METHOD PUBLIC Twice : INT
    Twice := LEN(THIS^.Name) * 2;
  END_METHOD
END_FUNCTION_BLOCK

TYPE St : STRUCT state : INT; n : INT; END_STRUCT; END_TYPE

FUNCTION_BLOCK Dev
  VAR _st : St; END_VAR
  PROPERTY PUBLIC Status : REFERENCE TO St
    GET
      Status REF= _st;
    END_GET
  END_PROPERTY
  METHOD PUBLIC Run
    Status.state := Status.state + 1;
  END_METHOD
END_FUNCTION_BLOCK

PROGRAM main
  VAR m : Motor; i : IHasLevel; nm : Named; d : Dev; END_VAR
  VAR q0 AT %QW0 : INT; q1 AT %QW2 : INT; q2 AT %QW4 : INT; q3 AT %QW6 : INT; q4 AT %QW8 : INT; q5 AT %QW10 : INT; q6 AT %QW12 : INT; END_VAR
  m.Speed := 150;
  q0 := m.Speed;
  m.Speed := 40;
  m.Boost();
  q1 := m.Speed;
  q2 := m.Sets;
  i := m;
  q3 := i.Level;
  nm.Name := 'xy';
  q4 := nm.Twice();
  d.Run();
  d.Run();
  d.Status.n := 7;
  q5 := d.Status.state;
  q6 := d.Status.n;
END_PROGRAM

CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION
