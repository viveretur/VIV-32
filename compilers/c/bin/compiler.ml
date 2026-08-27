let read_file filename =
  In_channel.with_open_bin filename In_channel.input_all

let write_file filename contents =
  Out_channel.with_open_bin filename (fun channel ->
    Out_channel.output_string channel contents)

let replace_extension filename extension =
  Filename.remove_extension filename ^ extension

let dump_tokens output_file tokens =
  let filename = replace_extension output_file ".tokens" in
  let contents =
    tokens
    |> List.map C.Token.serialize
    |> String.concat "\n"
  in
  write_file filename (contents ^ "\n")

let dump_ast output_file ast =
  let filename = replace_extension output_file ".ast" in
  write_file filename (C.Ast.string_of_program ast)

let input_file = ref None
let output_file = ref None
let should_dump_tokens = ref false
let should_dump_ast = ref false

let set_input filename =
  match !input_file with
  | None ->
      input_file := Some filename
  | Some _ ->
      raise (Arg.Bad "only one input file may be specified")

let options =
  [
    ( "-o",
      Arg.String (fun filename -> output_file := Some filename),
      "<file> Write compiled output to <file>" );

    ( "--dump-tokens",
      Arg.Set should_dump_tokens,
      " Write lexer output alongside the compiled output" );

    ( "--dump-ast",
      Arg.Set should_dump_ast,
      " Write parser AST alongside the compiled output" );
  ]

let usage =
  "usage: compiler [--dump-tokens] [--dump-ast] <input> -o <output>"

let () =
  Arg.parse options set_input usage;

  let input_file =
    match !input_file with
    | Some filename -> filename
    | None ->
        prerr_endline "compiler: no input file specified";
        Arg.usage options usage;
        exit 1
  in

  let output_file =
    match !output_file with
    | Some filename -> filename
    | None ->
        prerr_endline "compiler: no output file specified";
        Arg.usage options usage;
        exit 1
  in

  try
    let source = read_file input_file in

    let tokens = C.Lexer.lex source in

    if !should_dump_tokens then
      dump_tokens output_file tokens;

    let ast = C.Parser.parse tokens in

    if !should_dump_ast then
      dump_ast output_file ast;

    let output = C.Compiler.compile ast in

    write_file output_file output

  with
  | Sys_error message ->
      prerr_endline ("compiler: " ^ message);
      exit 1
