let read_file filename =
  In_channel.with_open_bin filename In_channel.input_all

let write_file filename contents =
  Out_channel.with_open_bin filename (fun channel ->
    Out_channel.output_string channel contents)

let () =
  if Array.length Sys.argv <> 4 || Sys.argv.(2) <> "-o" then begin
    prerr_endline "usage: lexer <input> -o <output>";
    exit 1
  end;

  let input_file = Sys.argv.(1) in
  let output_file = Sys.argv.(3) in
  ignore output_file;

  try
    let source = read_file input_file in
    let tokens = C.Lexer.lex source in
    let output =
      tokens
        |> List.map C.Token.serialize
        |> String.concat "\n"
    in
    write_file output_file (output ^ "\n")
    
  with
  | Sys_error message ->
      prerr_endline ("lexer: " ^ message);
      exit 1
