let compile_expression register = function
  | Ast.ConstantInt value ->
      Printf.sprintf "    li      $%d, %d\n" register value

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
