type kind =
  | Semicolon
  | ParenClose
  | ParenOpen
  | BraceClose
  | BraceOpen
  | OperatorComplement
  | OperatorDecrement
  | OperatorNegate
  | KeywordInt
  | KeywordReturn
  | Identifier of string
  | ConstantInt of int

type t = {
  kind : kind;
  line_no : int;
  column_no : int;
}

let serialize token =
  let prefix name =
    Printf.sprintf "%d %d %s"
      token.line_no token.column_no name
  in

  match token.kind with
    | Semicolon ->          prefix "SEMICOLON"
    | ParenClose ->         prefix "PAREN_CLOSE"
    | ParenOpen ->          prefix "PAREN_OPEN"
    | BraceClose ->         prefix "BRACE_CLOSE"
    | BraceOpen ->          prefix "BRACE_OPEN"
    | OperatorComplement -> prefix "OPERATOR_COMPLEMENT"
    | OperatorDecrement ->  prefix "OPERATOR_DECREMENT"
    | OperatorNegate ->     prefix "OPERATOR_NEGATE"
    | KeywordInt ->         prefix "INT"
    | KeywordReturn ->      prefix "RETURN"
    | Identifier value ->   Printf.sprintf "%s %s" (prefix "IDENTIFIER") value
    | ConstantInt value ->  Printf.sprintf "%s %d" (prefix "CONSTANT_INT") value

let split_field str pos = match String.index_from_opt str pos ' ' with
  | Some next ->  (String.sub str pos (next - pos), next + 1)
  | None ->       (String.sub str pos (String.length str - pos), String.length str)

let deserialize line =
  let line_no_str, pos = split_field line 0 in
  let column_no_str, pos = split_field line pos in
  let kind_str, pos = split_field line pos in

  let payload =
    if pos < String.length line then
      String.sub line pos (String.length line - pos)
    else
      ""
  in

  let line_no = int_of_string line_no_str in
  let column_no = int_of_string column_no_str in

  let kind = match kind_str with
    | "SEMICOLON" ->            Semicolon
    | "PAREN_OPEN" ->           ParenOpen
    | "PAREN_CLOSE" ->          ParenClose
    | "BRACE_OPEN" ->           BraceOpen
    | "BRACE_CLOSE" ->          BraceClose
    | "OPERATOR_COMPLEMENT" ->  OperatorComplement
    | "OPERATOR_DECREMENT" ->   OperatorDecrement
    | "OPERATOR_NEGATE" ->      OperatorNegate
    | "INT" ->                  KeywordInt
    | "RETURN" ->               KeywordReturn
    | "IDENTIFIER" ->           Identifier payload
    | "CONSTANT_INT" ->         ConstantInt (int_of_string payload)
    
    | other ->                  failwith ("Unknown token kind: " ^ other)
  in {
    kind;
    line_no;
    column_no;
  }
