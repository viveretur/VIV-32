let () =
  let token = {
    C.Token.kind = C.Token.Identifier "main";
    line_no = 1;
    column_no = 5;
  } in

  assert (C.Token.deserialize(C.Token.serialize token) = token)
