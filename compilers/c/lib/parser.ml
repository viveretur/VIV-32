let expect (tokens : Token.t array) index expected message =
  if tokens.(index).kind = expected then
    index + 1
  else
    failwith message


let expect_identifier (tokens : Token.t array) index =
  match tokens.(index).kind with
    | Token.Identifier name -> (index + 1, name)
    | _ -> failwith "Expected identifier"


let rec parse_expression (tokens : Token.t array) index =
  match tokens.(index).kind with
    | Token.ConstantInt value -> (index + 1, Ast.ConstantInt value)
    | Token.OperatorComplement ->
        let next, expression = parse_expression tokens (index + 1) in
        (next, Ast.Unary (Ast.OperatorComplement, expression))
    | Token.OperatorNegate ->
        let next, expression = parse_expression tokens (index + 1) in
        (next, Ast.Unary (Ast.OperatorNegate, expression))
    | Token.ParenOpen ->
        let next, expression = parse_expression tokens (index + 1) in
        begin
          match tokens.(next).kind with
            | Token.ParenClose -> (next + 1, expression)
            | _ -> failwith "Expected ')'"
        end
    | _ -> failwith "Expected expression"


let parse_statement (tokens : Token.t array) index =
  match tokens.(index).kind with
    | Token.KeywordReturn ->
        let next, expression = parse_expression tokens (index + 1) in
        begin
          match tokens.(next).kind with
            | Token.Semicolon -> (next + 1, Ast.Return expression)
            | _ -> failwith "Expected ';' after return expression"
        end
    | _ -> failwith "Expected statement"


let parse_function (tokens : Token.t array) index =
  let index = expect tokens index Token.KeywordInt "Expected 'int'" in

  let index, name = expect_identifier tokens index in
  let index = expect tokens index Token.ParenOpen "Expected '('" in
  let index = expect tokens index Token.ParenClose "Expected ')'" in
  let index = expect tokens index Token.BraceOpen "Expected '{'" in
  let index, statement = parse_statement tokens index in
  let index = expect tokens index Token.BraceClose "Expected '}'" in

  (index, {
    Ast.name;
    body = statement;
  })


let parse_program tokens index =
  let next, function_definition =
    parse_function tokens index
  in

  (next, Ast.Program function_definition)


let parse tokens =
  let tokens = Array.of_list tokens in
  let next, program = parse_program tokens 0 in

  if next <> Array.length tokens then
    failwith "Unexpected tokens after program"
  else
    program
