(* CODESYS's and TwinCAT's __VARINFO (var-info): a __SYSTEM.VAR_INFO of
   a variable, as TcOpen's tasks name the signal they wait for in a
   message (`__VARINFO(inoStatus.State...).Symbol`).
   Worked by hand, the same each tick:
   %QB0 LEN(msg), 'waiting for ' (12) + 'ES_IsStopped' (12): 24;
   %QB1 BitSize of a BOOL, a byte: 8;
   %QB2 TypeClass of io.Vals is TYPE_ARRAY: 1;
   %QB3 NumElements of ARRAY[1..3] OF INT: 3;
   %QB4 ElemBitSize, an INT: 16;
   %QB5 BaseTypeClass is TYPE_INT: 1;
   %QB6 LEN(TypeName), 'ARRAY[1..3] OF INT': 18;
   %QB7 BitNo of w.5: 5; %QB8 its BitSize: 1;
   %QB9 MemoryArea, CODESYS's initial MEM_MEMORY: 1.
   reference-skip: not IEC 61131-3. *)
TYPE Sig : STRUCT
  ES_IsStopped : BOOL;
  Vals : ARRAY[1..3] OF INT;
END_STRUCT;
END_TYPE

FUNCTION_BLOCK Watch
  VAR_IN_OUT io : Sig; END_VAR
  VAR_OUTPUT msg : STRING[60]; bits : INT; END_VAR
  VAR info : __SYSTEM.VAR_INFO; END_VAR
  info := __VARINFO(io.ES_IsStopped);
  msg := CONCAT('waiting for ', info.Symbol);
  bits := info.BitSize;
END_FUNCTION_BLOCK

PROGRAM main
  VAR s : Sig; wt : Watch; info : __SYSTEM.VAR_INFO; w : WORD; END_VAR
  VAR q AT %QB0 : ARRAY[0..9] OF BYTE; END_VAR
  wt(io := s);
  q[0] := TO_BYTE(LEN(wt.msg));
  q[1] := TO_BYTE(wt.bits);
  info := __VARINFO(s.Vals);
  q[2] := BOOL_TO_BYTE(info.TypeClass = __SYSTEM.TYPE_CLASS.TYPE_ARRAY);
  q[3] := TO_BYTE(info.NumElements);
  q[4] := TO_BYTE(info.ElemBitSize);
  q[5] := BOOL_TO_BYTE(info.BaseTypeClass = __SYSTEM.TYPE_CLASS.TYPE_INT);
  q[6] := TO_BYTE(LEN(info.TypeName));
  info := __VARINFO(w.5);
  q[7] := TO_BYTE(info.BitNo);
  q[8] := TO_BYTE(info.BitSize);
  q[9] := BOOL_TO_BYTE(info.MemoryArea = __SYSTEM.MEMORY_AREA.MEM_MEMORY);
END_PROGRAM

CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION
