let scan_placeholder (_source : string) index = (index + 1, Token.Semicolon)


let scan_identifier_keyword source start =
  let len = String.length source in

  let rec find_end index =
    if index < len then
      match source.[index] with
      | 'a'..'z'
      | 'A'..'Z'
      | '0'..'9'
      | '_' -> find_end (index + 1)
    
      | _ -> index
    else
      index
  in

  let finish = find_end (start + 1) in
  let text = String.sub source start (finish - start) in

  let kind =
    match text with
      | "int" -> Token.KeywordInt
      | "return" -> Token.KeywordReturn
      | _ -> Token.Identifier text
  in
  (finish, kind)


let scan_constant_number source start =
  let len = String.length source in

  let rec find_end index =
    if index < len then
      match source.[index] with
      | '0'..'9' -> find_end (index + 1)
      | _ -> index
    else
      index
  in

  let finish = find_end (start + 1) in
  let text = String.sub source start (finish - start) in

  (finish, Token.ConstantInt (int_of_string text))


let scan_negate source start =
  let len = String.length source in

  let rec find_end index =
    if index < len then
      match source.[index] with
        | '-' -> find_end (index + 1)
        | _ -> index
      else
        index
  in

  let finish = find_end (start + 1) in
  let text = String.sub source start (finish - start) in

  let kind = match text with
    | "-" -> Token.OperatorNegate
    | "--" -> Token.OperatorDecrement
    | x -> failwith ("Unsupported token:" ^ x)
  in
  (finish, kind)
    

let lex source =
  let rec loop index line_no line_start tokens =
    if index >= String.length source then
      List.rev tokens
    else
      match source.[index] with
        | '\n' -> loop (index + 1) (line_no + 1) (index + 1) tokens

        | ' '
        | '\t'
        | '\r' -> loop (index + 1) line_no line_start tokens

        | ';' -> let token = {
                  Token.kind = Token.Semicolon;
                  line_no;
                  column_no = index - line_start + 1;
              } in loop (index + 1) line_no line_start (token :: tokens)

        | '(' -> let token = {
                  Token.kind = Token.ParenOpen;
                  line_no;
                  column_no = index - line_start + 1;
              } in loop (index + 1) line_no line_start (token :: tokens)

        | ')' -> let token = {
                  Token.kind = Token.ParenClose;
                  line_no;
                  column_no = index - line_start + 1;
              } in loop (index + 1) line_no line_start (token :: tokens)

        | '{' -> let token = {
                  Token.kind = Token.BraceOpen;
                  line_no;
                  column_no = index - line_start + 1;
              } in loop (index + 1) line_no line_start (token :: tokens)

        | '}' -> let token = {
                  Token.kind = Token.BraceClose;
                  line_no;
                  column_no = index - line_start + 1;
              } in loop (index + 1) line_no line_start (token :: tokens)

        | '~' -> let token = {
                  Token.kind = Token.OperatorComplement;
                  line_no;
                  column_no = index - line_start + 1;
              } in loop (index + 1) line_no line_start (token :: tokens)

        | '-' ->
              let next, kind = scan_negate source index in
              let token = {
                  Token.kind = Token.OperatorNegate;
                  line_no;
                  column_no = index - line_start + 1;
              } in loop next line_no line_start (token :: tokens)

        | '0'..'9' ->
              let next, kind = scan_constant_number source index in
              let token = {
                Token.kind;
                line_no;
                column_no = index - line_start + 1;
              } in loop next line_no line_start (token :: tokens)
  
        | 'a'..'z'
        | 'A'..'Z'
        | '_' ->
              let next, kind = scan_identifier_keyword source index in
              let token = {
                Token.kind;
                line_no;
                column_no = index - line_start + 1;
              } in loop next line_no line_start (token :: tokens)
  
        | _ -> loop (index + 1) line_no line_start tokens
  in

  loop 0 1 0 []
