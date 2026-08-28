(* For pretty printing *)
let indent depth = String.make (depth * 2) ' '


(** Unary operators *)
type unary_operator =
  | OperatorComplement
  | OperatorNegate

let string_of_unary_operator depth = function
  | OperatorComplement -> Printf.sprintf "%sOperatorComplement" (String.make depth ' ')
  | OperatorNegate ->     Printf.sprintf "%sOperatorNegate" (String.make depth ' ')


(** Expression rules *)
type expression =
  | ConstantInt of int
  | Unary of unary_operator * expression

let rec string_of_expression depth = function
  | ConstantInt value -> Printf.sprintf "%sConstantInt %d\n" (String.make depth ' ') value
  | Unary (operator, expression) ->
      Printf.sprintf "%sUnary\n%s\n%s"
        (String.make depth ' ')
        (string_of_unary_operator (depth + 2) operator)
        (string_of_expression (depth + 2) expression)
    

(** Statement rules *)
type statement =
  | Return of expression

let string_of_statement depth = function
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
