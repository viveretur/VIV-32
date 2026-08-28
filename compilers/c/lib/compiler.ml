let rec compile_expression register = function
  | Ast.ConstantInt value ->
      Printf.sprintf "    li      $%d, %d\n" register value
  | Ast.Unary (operator, expression) ->
      let expression_code = compile_expression register expression in
      let operator_code =
        match operator with
          | Ast.OperatorComplement ->
              Printf.sprintf "    not     $%d, $%d\n" register register
          | Ast.OperatorNegate ->
              Printf.sprintf "    neg     $%d, $%d\n" register register
      in
      expression_code ^ operator_code

let compile_statement = function
  | Ast.Return expr ->
      (* Register 2 is currently used for returns.  *)
      compile_expression 2 expr ^
      "    ret\n"

let compile_function fn =
  Printf.sprintf ".nomangle %s\n%s:\n"
    fn.Ast.name
    fn.name
  ^
  compile_statement fn.body

let compile = function
  | Ast.Program fn ->
      compile_function fn
