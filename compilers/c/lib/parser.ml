let parse_expression (tokens : Token.t array) index =
  match tokens.(index).kind with
    | Token.ConstantInt value -> (index + 1, Ast.ConstantInt value)
    | _ -> failwith "Expected expression"


let parse_statement (tokens : Token.t array) index =
  match tokens.(index).kind with
  | Token.KeywordReturn ->
      let next, expression =
        parse_expression tokens (index + 1)
      in

      begin
        match tokens.(next).kind with
        | Token.Semicolon ->
            (next + 1, Ast.Return expression)
        | _ ->
            failwith "Expected ';' after return expression"
      end

  | _ ->
      failwith "Expected statement"

  
let parse_function (tokens : Token.t array) index =
  match tokens.(index).kind with
  | Token.KeywordInt ->
      begin
        match tokens.(index + 1).kind with
        | Token.Identifier name ->
            begin
              match tokens.(index + 2).kind with
              | Token.ParenOpen ->
                  begin
                    match tokens.(index + 3).kind with
                    | Token.ParenClose ->
                        begin
                          match tokens.(index + 4).kind with
                          | Token.BraceOpen ->
                              let next, statement =
                                parse_statement tokens (index + 5)
                              in

                              begin
                                match tokens.(next).kind with
                                | Token.BraceClose ->
                                    (next + 1,
                                     {
                                       Ast.name;
                                       body = statement;
                                     })
                                | _ ->
                                    failwith "Expected '}'"
                              end

                          | _ ->
                              failwith "Expected '{'"
                        end

                    | _ ->
                        failwith "Expected ')'"
                  end

              | _ ->
                  failwith "Expected '('"
            end

        | _ ->
            failwith "Expected function name"
      end

  | _ ->
      failwith "Expected 'int'"


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
