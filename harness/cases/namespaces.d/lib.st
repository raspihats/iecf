(* The other file of the namespaces case: more of Lib. *)
NAMESPACE Lib
    NAMESPACE Shapes
        TYPE Corner : (Sharp, Round); END_TYPE

        FUNCTION area : INT
        VAR_INPUT w, h : INT; END_VAR
            area := w * h + offset();
        END_FUNCTION

        FUNCTION INTERNAL offset : INT
            offset := 100;
        END_FUNCTION
    END_NAMESPACE

    NAMESPACE INTERNAL Detail
        FUNCTION scale : INT
        VAR_INPUT x : INT; END_VAR
            scale := x * 10;
        END_FUNCTION
    END_NAMESPACE

    FUNCTION tenfold : INT
    VAR_INPUT x : INT; END_VAR
        tenfold := Detail.scale(x);
    END_FUNCTION
END_NAMESPACE
