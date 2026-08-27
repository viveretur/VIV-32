(* For pretty printing *)
let indent depth = String.make (depth * 2) ' '

(** Expression rules *)
type expression =
  | ConstantInt of int

let rec string_of_expression depth = function
  | ConstantInt value ->
      Printf.sprintf "%sConstantInt %d\n"
        (indent depth) value


(** Statement rules *)
type statement =
  | Return of expression

let rec string_of_statement depth = function
  | Return expr ->
      Printf.sprintf "%sReturn\n%s"
        (indent depth)
        (string_of_expression (depth + 1) expr)


(** Function rules *)
type function_definition = {
  name : string;
  body : statement;
}

let string_of_function depth fn =
  Printf.sprintf "%sFunction %s\n%s"
    (indent depth)
    fn.name
    (string_of_statement (depth + 1) fn.body)


(** Program rules *)
type program =
  | Program of function_definition

let string_of_program = function
  | Program fn ->
      Printf.sprintf "Program\n%s"
        (string_of_function 1 fn)
