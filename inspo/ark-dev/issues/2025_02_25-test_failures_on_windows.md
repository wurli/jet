# Test failures on Windows

> <https://github.com/posit-dev/ark/issues/678>
> 
> * Author: @DavisVaughan
> * State: CLOSED
> * Labels: 

```
thread 'lsp::completions::sources::composite::call::tests::test_completions_after_user_types_part_of_an_argument_name' panicked at core\src\panicking.rs:223:5:
panic in a function that cannot unwind
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
thread caused non-unwinding panic. aborting.
error: test failed, to rerun pass `-p ark --lib`
```

Note that the specific test you see there is a red herring and that's just the _first_ of many tests that are failing. Here is a more informative bit of output from nextest from some testing in #676:

<details>

```
       ABORT [   0.560s] harp exec::tests::test_top_level_exec
           - with code 0xc0000409: The system detected an overrun of a stack-based buffer in this application. This overrun could potentially allow a malicious user to gain control of this application. (os error 1282)
       START             harp exec::tests::test_try_catch_error

running 1 test
thread 'exec::tests::test_try_catch_error' panicked at core\src\panicking.rs:223:5:
panic in a function that cannot unwind
stack backtrace:
   0: std::panicking::begin_panic_handler
             at /rustc/9fc6b43126469e3858e2fe86cafb4f0fd5068869\library/std\src\panicking.rs:665
   1: core::panicking::panic_nounwind_fmt
             at /rustc/9fc6b43126469e3858e2fe86cafb4f0fd5068869\library/core\src\intrinsics\mod.rs:3535
   2: core::panicking::panic_nounwind
             at /rustc/9fc6b43126469e3858e2fe86cafb4f0fd5068869\library/core\src\panicking.rs:223
   3: core::panicking::panic_cannot_unwind
             at /rustc/9fc6b43126469e3858e2fe86cafb4f0fd5068869\library/core\src\panicking.rs:315
   4: harp::exec::try_catch::callback<harp::exec::tests::test_try_catch_error::closure$0::closure_env$1,tuple$<> >
             at .\src\exec.rs:189
   5: _CxxFrameHandler3
   6: is_exception_typeof
   7: _C_specific_handler
   8: is_exception_typeof
   9: _CxxFrameHandler3
  10: _chkstk
  11: RtlUnwindEx
  12: RtlUnwind
  13: _intrinsic_setjmpex
  14: R_new_custom_connection
  15: R_CheckStack2
  16: Rf_jump_to_toplevel
  17: Rf_jump_to_toplevel
  18: Rf_jump_to_toplevel
  19: R_ParseEvalString
  20: R_ParseEvalString
  21: R_ParseEvalString
  22: Rf_eval
  23: Rf_eval
  24: R_ParseEvalString
  25: libr::r::Rf_eval
             at D:\a\ark\ark\crates\libr\src\functions.rs:31
  26: harp::exec::try_catch::handler<harp::exec::tests::test_try_catch_error::closure$0::closure_env$1,tuple$<> >
             at .\src\exec.rs:266
  27: do_Rprofmem
  28: R_ParseEvalString
  29: Rf_eval
  30: Rf_eval
  31: R_ParseEvalString
  32: R_ParseEvalString
  33: R_ParseEvalString
  34: Rf_eval
  35: Rf_eval
  36: R_ParseEvalString
  37: Rf_error
  38: Rf_errorcall
  39: Rf_error
  40: libr::r::Rf_error
             at D:\a\ark\ark\crates\libr\src\functions_variadic.rs:33
  41: harp::exec::tests::test_try_catch_error::closure$0::closure$1
             at .\src\exec.rs:574
  42: harp::exec::try_catch::callback<harp::exec::tests::test_try_catch_error::closure$0::closure_env$1,tuple$<> >
             at .\src\exec.rs:200
  43: R_withCallingErrorHandler
  44: libr::r::R_withCallingErrorHandler
             at D:\a\ark\ark\crates\libr\src\functions.rs:31
  45: harp::exec::try_catch::closure$0<harp::exec::tests::test_try_catch_error::closure$0::closure_env$1,tuple$<> >
             at .\src\exec.rs:272
  46: harp::exec::top_level_exec::callback<harp::exec::try_catch::closure_env$0<harp::exec::tests::test_try_catch_error::closure$0::closure_env$1,tuple$<> >,tuple$<> >
             at .\src\exec.rs:342
  47: R_ToplevelExec
  48: libr::r::R_ToplevelExec
             at D:\a\ark\ark\crates\libr\src\functions.rs:31
  49: harp::exec::top_level_exec<harp::exec::try_catch::closure_env$0<harp::exec::tests::test_try_catch_error::closure$0::closure_env$1,tuple$<> >,tuple$<> >
             at .\src\exec.rs:345
  50: harp::exec::try_catch<harp::exec::tests::test_try_catch_error::closure$0::closure_env$1,tuple$<> >
             at .\src\exec.rs:271
  51: harp::exec::tests::test_try_catch_error::closure$0
             at .\src\exec.rs:571
  52: harp::fixtures::r_task<harp::exec::tests::test_try_catch_error::closure_env$0>
             at .\src\fixtures\mod.rs:41
  53: harp::exec::tests::test_try_catch_error
             at .\src\exec.rs:562
  54: harp::exec::tests::test_try_catch_error::closure$0
             at .\src\exec.rs:561
  55: core::ops::function::FnOnce::call_once<harp::exec::tests::test_try_catch_error::closure_env$0,tuple$<> >
             at /rustc/9fc6b43126469e3858e2fe86cafb4f0fd5068869\library\core\src\ops\function.rs:250
  56: core::ops::function::FnOnce::call_once
             at /rustc/9fc6b43126469e3858e2fe86cafb4f0fd5068869\library/core\src\ops\function.rs:250
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
thread caused non-unwinding panic. aborting.
       ABORT [   0.531s] harp exec::tests::test_try_catch_error
           - with code 0xc0000409: The system detected an overrun of a stack-based buffer in this application. This overrun could potentially allow a malicious user to gain control of this application. (os error 1282)
       START             harp exec::tests::test_utf8_strings

running 1 test
test exec::tests::test_utf8_strings ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 88 filtered out; finished in 0.17s

        PASS [   0.181s] harp exec::tests::test_utf8_strings
       START             harp fixtures::tests::test_stack_info

running 1 test
test fixtures::tests::test_stack_info ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 88 filtered out; finished in 0.17s

        PASS [   0.183s] harp fixtures::tests::test_stack_info
       START             harp format::tests::test_to_string_methods

running 1 test
test format::tests::test_to_string_methods ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 88 filtered out; finished in 0.17s

        PASS [   0.178s] harp format::tests::test_to_string_methods
       START             harp json::tests::test_json_lists_duplicate

running 1 test
test json::tests::test_json_lists_duplicate ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 88 filtered out; finished in 0.17s

        PASS [   0.180s] harp json::tests::test_json_lists_duplicate
       START             harp json::tests::test_json_lists_mixed_types

running 1 test
test json::tests::test_json_lists_mixed_types ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 88 filtered out; finished in 0.16s

        PASS [   0.175s] harp json::tests::test_json_lists_mixed_types
       START             harp json::tests::test_json_lists_named

running 1 test
test json::tests::test_json_lists_named ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 88 filtered out; finished in 0.16s

        PASS [   0.175s] harp json::tests::test_json_lists_named
       START             harp json::tests::test_json_lists_nested

running 1 test
test json::tests::test_json_lists_nested ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 88 filtered out; finished in 0.17s

        PASS [   0.177s] harp json::tests::test_json_lists_nested
       START             harp json::tests::test_json_lists_unnamed

running 1 test
test json::tests::test_json_lists_unnamed ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 88 filtered out; finished in 0.17s

        PASS [   0.178s] harp json::tests::test_json_lists_unnamed
       START             harp json::tests::test_json_na_vectors

running 1 test
test json::tests::test_json_na_vectors ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 88 filtered out; finished in 0.17s

        PASS [   0.177s] harp json::tests::test_json_na_vectors
       START             harp json::tests::test_json_scalars

running 1 test
test json::tests::test_json_scalars ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 88 filtered out; finished in 0.17s

        PASS [   0.176s] harp json::tests::test_json_scalars
       START             harp json::tests::test_json_vectors

running 1 test
test json::tests::test_json_vectors ... ok

        PASS [   0.176s] harp json::tests::test_json_vectors
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 88 filtered out; finished in 0.17s

       START             harp json::tests::test_r_to_json_lists

running 1 test
test json::tests::test_r_to_json_lists ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 88 filtered out; finished in 0.17s

        PASS [   0.176s] harp json::tests::test_r_to_json_lists
       START             harp json::tests::test_r_to_json_lists_mixed_types

running 1 test
test json::tests::test_r_to_json_lists_mixed_types ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 88 filtered out; finished in 0.17s

        PASS [   0.176s] harp json::tests::test_r_to_json_lists_mixed_types
       START             harp json::tests::test_r_to_json_objects

running 1 test
test json::tests::test_r_to_json_objects ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 88 filtered out; finished in 0.17s

        PASS [   0.178s] harp json::tests::test_r_to_json_objects
       START             harp json::tests::test_r_to_json_scalars

running 1 test
test json::tests::test_r_to_json_scalars ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 88 filtered out; finished in 0.17s

        PASS [   0.179s] harp json::tests::test_r_to_json_scalars
       START             harp line_ending::test_convert_line_endings_explicit

running 1 test
test line_ending::test_convert_line_endings_explicit ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 88 filtered out; finished in 0.00s

        PASS [   0.006s] harp line_ending::test_convert_line_endings_explicit
       START             harp object::tests::test_tryfrom_RObject_Option_String

running 1 test
test object::tests::test_tryfrom_RObject_Option_String ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 88 filtered out; finished in 0.17s

        PASS [   0.176s] harp object::tests::test_tryfrom_RObject_Option_String
       START             harp object::tests::test_tryfrom_RObject_String

running 1 test
test object::tests::test_tryfrom_RObject_String ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 88 filtered out; finished in 0.17s

        PASS [   0.178s] harp object::tests::test_tryfrom_RObject_String
       START             harp object::tests::test_tryfrom_RObject_Vec_Bool

running 1 test
test object::tests::test_tryfrom_RObject_Vec_Bool ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 88 filtered out; finished in 0.17s

        PASS [   0.180s] harp object::tests::test_tryfrom_RObject_Vec_Bool
       START             harp object::tests::test_tryfrom_RObject_Vec_Option_String

running 1 test
test object::tests::test_tryfrom_RObject_Vec_Option_String ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 88 filtered out; finished in 0.17s

        PASS [   0.179s] harp object::tests::test_tryfrom_RObject_Vec_Option_String
       START             harp object::tests::test_tryfrom_RObject_Vec_RObject

running 1 test
test object::tests::test_tryfrom_RObject_Vec_RObject ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 88 filtered out; finished in 0.18s

        PASS [   0.187s] harp object::tests::test_tryfrom_RObject_Vec_RObject
       START             harp object::tests::test_tryfrom_RObject_Vec_String

running 1 test
test object::tests::test_tryfrom_RObject_Vec_String ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 88 filtered out; finished in 0.17s

        PASS [   0.176s] harp object::tests::test_tryfrom_RObject_Vec_String
       START             harp object::tests::test_tryfrom_RObject_Vec_i32

running 1 test
test object::tests::test_tryfrom_RObject_Vec_i32 ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 88 filtered out; finished in 0.16s

        PASS [   0.175s] harp object::tests::test_tryfrom_RObject_Vec_i32
       START             harp object::tests::test_tryfrom_RObject_bool

running 1 test
test object::tests::test_tryfrom_RObject_bool ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 88 filtered out; finished in 0.17s

        PASS [   0.179s] harp object::tests::test_tryfrom_RObject_bool
       START             harp object::tests::test_tryfrom_RObject_f64

running 1 test
test object::tests::test_tryfrom_RObject_f64 ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 88 filtered out; finished in 0.18s

        PASS [   0.189s] harp object::tests::test_tryfrom_RObject_f64
       START             harp object::tests::test_tryfrom_RObject_hashmap_Robject

running 1 test
test object::tests::test_tryfrom_RObject_hashmap_Robject ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 88 filtered out; finished in 0.17s

        PASS [   0.178s] harp object::tests::test_tryfrom_RObject_hashmap_Robject
       START             harp object::tests::test_tryfrom_RObject_hashmap_i32

running 1 test
test object::tests::test_tryfrom_RObject_hashmap_i32 ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 88 filtered out; finished in 0.17s

        PASS [   0.178s] harp object::tests::test_tryfrom_RObject_hashmap_i32
       START             harp object::tests::test_tryfrom_RObject_hashmap_string

running 1 test
test object::tests::test_tryfrom_RObject_hashmap_string ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 88 filtered out; finished in 0.17s

        PASS [   0.183s] harp object::tests::test_tryfrom_RObject_hashmap_string
       START             harp object::tests::test_tryfrom_RObject_i32

running 1 test
test object::tests::test_tryfrom_RObject_i32 ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 88 filtered out; finished in 0.17s

        PASS [   0.182s] harp object::tests::test_tryfrom_RObject_i32
       START             harp object::tests::test_tryfrom_RObject_u16

running 1 test
test object::tests::test_tryfrom_RObject_u16 ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 88 filtered out; finished in 0.17s

        PASS [   0.180s] harp object::tests::test_tryfrom_RObject_u16
       START             harp object::tests::test_tryfrom_Vec_RObject_RObject

running 1 test
test object::tests::test_tryfrom_Vec_RObject_RObject ... ok

        PASS [   0.180s] harp object::tests::test_tryfrom_Vec_RObject_RObject
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 88 filtered out; finished in 0.17s

       START             harp parse::tests::test_parse_input_as_string

running 1 test
test parse::tests::test_parse_input_as_string ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 88 filtered out; finished in 0.17s

        PASS [   0.178s] harp parse::tests::test_parse_input_as_string
       START             harp parse::tests::test_parse_status

running 1 test
thread 'parse::tests::test_parse_status' panicked at core\src\panicking.rs:223:5:
panic in a function that cannot unwind
stack backtrace:
   0: std::panicking::begin_panic_handler
             at /rustc/9fc6b43126469e3858e2fe86cafb4f0fd5068869\library/std\src\panicking.rs:665
   1: core::panicking::panic_nounwind_fmt
             at /rustc/9fc6b43126469e3858e2fe86cafb4f0fd5068869\library/core\src\intrinsics\mod.rs:3535
   2: core::panicking::panic_nounwind
             at /rustc/9fc6b43126469e3858e2fe86cafb4f0fd5068869\library/core\src\panicking.rs:223
   3: core::panicking::panic_cannot_unwind
             at /rustc/9fc6b43126469e3858e2fe86cafb4f0fd5068869\library/core\src\panicking.rs:315
   4: harp::exec::try_catch::callback<harp::parse::parse_status::closure_env$0,harp::object::RObject>
             at .\src\exec.rs:189
   5: _CxxFrameHandler3
   6: is_exception_typeof
   7: _C_specific_handler
   8: is_exception_typeof
   9: _CxxFrameHandler3
  10: _chkstk
  11: RtlUnwindEx
  12: RtlUnwind
  13: _intrinsic_setjmpex
  14: R_new_custom_connection
  15: R_CheckStack2
  16: Rf_jump_to_toplevel
  17: Rf_jump_to_toplevel
  18: Rf_jump_to_toplevel
  19: R_ParseEvalString
  20: R_ParseEvalString
  21: R_ParseEvalString
  22: Rf_eval
  23: Rf_eval
  24: R_ParseEvalString
  25: libr::r::Rf_eval
             at D:\a\ark\ark\crates\libr\src\functions.rs:31
  26: harp::exec::try_catch::handler<harp::parse::parse_status::closure_env$0,harp::object::RObject>
             at .\src\exec.rs:266
  27: do_Rprofmem
  28: R_ParseEvalString
  29: Rf_eval
  30: Rf_eval
  31: R_ParseEvalString
  32: R_CheckUserInterrupt
  33: R_CheckUserInterrupt
  34: R_CheckUserInterrupt
  35: Rf_doesIdle
  36: Rf_doesIdle
  37: Rf_doesIdle
  38: R_ParseVector
  39: libr::r::R_ParseVector
             at D:\a\ark\ark\crates\libr\src\functions.rs:31
  40: harp::parse::parse_status::closure$0
             at .\src\parse.rs:113
  41: harp::exec::try_catch::callback<harp::parse::parse_status::closure_env$0,harp::object::RObject>
             at .\src\exec.rs:200
  42: R_withCallingErrorHandler
  43: libr::r::R_withCallingErrorHandler
             at D:\a\ark\ark\crates\libr\src\functions.rs:31
  44: harp::exec::try_catch::closure$0<harp::parse::parse_status::closure_env$0,harp::object::RObject>
             at .\src\exec.rs:272
  45: harp::exec::top_level_exec::callback<harp::exec::try_catch::closure_env$0<harp::parse::parse_status::closure_env$0,harp::object::RObject>,tuple$<> >
             at .\src\exec.rs:342
  46: R_ToplevelExec
  47: libr::r::R_ToplevelExec
             at D:\a\ark\ark\crates\libr\src\functions.rs:31
  48: harp::exec::top_level_exec<harp::exec::try_catch::closure_env$0<harp::parse::parse_status::closure_env$0,harp::object::RObject>,tuple$<> >
             at .\src\exec.rs:345
  49: harp::exec::try_catch<harp::parse::parse_status::closure_env$0,harp::object::RObject>
             at .\src\exec.rs:271
  50: harp::parse::parse_status
             at .\src\parse.rs:113
  51: harp::parse::tests::test_parse_status::closure$0
             at .\src\parse.rs:209
  52: harp::fixtures::r_task<harp::parse::tests::test_parse_status::closure_env$0>
             at .\src\fixtures\mod.rs:41
  53: harp::parse::tests::test_parse_status
             at .\src\parse.rs:175
  54: harp::parse::tests::test_parse_status::closure$0
             at .\src\parse.rs:174
  55: core::ops::function::FnOnce::call_once<harp::parse::tests::test_parse_status::closure_env$0,tuple$<> >
             at /rustc/9fc6b43126469e3858e2fe86cafb4f0fd5068869\library\core\src\ops\function.rs:250
  56: core::ops::function::FnOnce::call_once
             at /rustc/9fc6b43126469e3858e2fe86cafb4f0fd5068869\library/core\src\ops\function.rs:250
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
thread caused non-unwinding panic. aborting.
       ABORT [   0.522s] harp parse::tests::test_parse_status
           - with code 0xc0000409: The system detected an overrun of a stack-based buffer in this application. This overrun could potentially allow a malicious user to gain control of this application. (os error 1282)
       START             harp parser::parse_data::tests::test_parse_data

running 1 test
test parser::parse_data::tests::test_parse_data ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 88 filtered out; finished in 0.17s

        PASS [   0.184s] harp parser::parse_data::tests::test_parse_data
       START             harp parser::srcref::tests::test_srcref

running 1 test
test parser::srcref::tests::test_srcref ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 88 filtered out; finished in 0.17s

        PASS [   0.178s] harp parser::srcref::tests::test_srcref
       START             harp parser::srcref::tests::test_srcref_line_directive

running 1 test
test parser::srcref::tests::test_srcref_line_directive ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 88 filtered out; finished in 0.17s

        PASS [   0.176s] harp parser::srcref::tests::test_srcref_line_directive
       START             harp raii::tests::test_local_option

running 1 test
test raii::tests::test_local_option ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 88 filtered out; finished in 0.17s

        PASS [   0.177s] harp raii::tests::test_local_option
       START             harp raii::tests::test_local_variable

running 1 test
test raii::tests::test_local_variable ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 88 filtered out; finished in 0.17s

        PASS [   0.179s] harp raii::tests::test_local_variable
       START             harp size::tests::test_compute_size_defered_strings

running 1 test
test size::tests::test_compute_size_defered_strings ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 88 filtered out; finished in 0.17s

        PASS [   0.178s] harp size::tests::test_compute_size_defered_strings
       START             harp size::tests::test_duplicated_charsxps_counted_once

running 1 test
test size::tests::test_duplicated_charsxps_counted_once ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 88 filtered out; finished in 0.17s

        PASS [   0.179s] harp size::tests::test_duplicated_charsxps_counted_once
       START             harp size::tests::test_env_size_recursive

running 1 test
test size::tests::test_env_size_recursive ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 88 filtered out; finished in 0.17s

        PASS [   0.180s] harp size::tests::test_env_size_recursive
       START             harp size::tests::test_immediate_bindings

running 1 test
test size::tests::test_immediate_bindings ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 88 filtered out; finished in 0.18s

        PASS [   0.191s] harp size::tests::test_immediate_bindings
       START             harp size::tests::test_length_one_vectors

running 1 test
test size::tests::test_length_one_vectors ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 88 filtered out; finished in 0.17s

        PASS [   0.179s] harp size::tests::test_length_one_vectors
       START             harp size::tests::test_pairlists

running 1 test
test size::tests::test_pairlists ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 88 filtered out; finished in 0.17s

        PASS [   0.178s] harp size::tests::test_pairlists
       START             harp size::tests::test_s4_classes

running 1 test
test size::tests::test_s4_classes ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 88 filtered out; finished in 0.17s

        PASS [   0.184s] harp size::tests::test_s4_classes
       START             harp size::tests::test_shared_components_once

running 1 test
test size::tests::test_shared_components_once ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 88 filtered out; finished in 0.17s

        PASS [   0.177s] harp size::tests::test_shared_components_once
       START             harp size::tests::test_size_attributes

running 1 test
test size::tests::test_size_attributes ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 88 filtered out; finished in 0.17s

        PASS [   0.177s] harp size::tests::test_size_attributes
       START             harp size::tests::test_size_closures

running 1 test
test size::tests::test_size_closures ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 88 filtered out; finished in 0.17s
        PASS [   0.177s] harp size::tests::test_size_closures
       START             harp size::tests::test_size_of_functions_include_envs


running 1 test
test size::tests::test_size_of_functions_include_envs ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 88 filtered out; finished in 0.17s

        PASS [   0.178s] harp size::tests::test_size_of_functions_include_envs
       START             harp size::tests::test_size_of_lists

running 1 test
test size::tests::test_size_of_lists ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 88 filtered out; finished in 0.17s

        PASS [   0.177s] harp size::tests::test_size_of_lists
       START             harp size::tests::test_size_of_symbols

running 1 test
test size::tests::test_size_of_symbols ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 88 filtered out; finished in 0.17s

        PASS [   0.179s] harp size::tests::test_size_of_symbols
       START             harp size::tests::test_sizes_scale_correctly

running 1 test
test size::tests::test_sizes_scale_correctly ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 88 filtered out; finished in 0.17s

        PASS [   0.180s] harp size::tests::test_sizes_scale_correctly
       START             harp size::tests::test_support_dots

running 1 test
test size::tests::test_support_dots ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 88 filtered out; finished in 0.17s

        PASS [   0.178s] harp size::tests::test_support_dots
       START             harp size::tests::test_terminal_envs_have_size_zero

running 1 test
test size::tests::test_terminal_envs_have_size_zero ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 88 filtered out; finished in 0.17s

        PASS [   0.179s] harp size::tests::test_terminal_envs_have_size_zero
       START             harp size::tests::test_works_for_altrep

running 1 test
test size::tests::test_works_for_altrep ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 88 filtered out; finished in 0.17s

        PASS [   0.177s] harp size::tests::test_works_for_altrep
       START             harp symbol::tests::test_rsymbol_ord

running 1 test
test symbol::tests::test_rsymbol_ord ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 88 filtered out; finished in 0.17s

        PASS [   0.182s] harp symbol::tests::test_rsymbol_ord
       START             harp sys::windows::line_ending::test_convert_line_endings_native_windows

running 1 test
test sys::windows::line_ending::test_convert_line_endings_native_windows ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 88 filtered out; finished in 0.00s

        PASS [   0.006s] harp sys::windows::line_ending::test_convert_line_endings_native_windows
       START             harp sys::windows::locale::tests::test_locale

running 1 test
test sys::windows::locale::tests::test_locale ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 88 filtered out; finished in 0.17s

        PASS [   0.179s] harp sys::windows::locale::tests::test_locale
       START             harp tests::test_call

running 1 test
test tests::test_call ... ok

        PASS [   0.178s] harp tests::test_call
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 88 filtered out; finished in 0.17s

       START             harp tests::test_pairlist

running 1 test
test tests::test_pairlist ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 88 filtered out; finished in 0.16s

        PASS [   0.175s] harp tests::test_pairlist
       START             harp traits::slice::test::test_slice

running 1 test
test traits::slice::test::test_slice ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 88 filtered out; finished in 0.00s

        PASS [   0.006s] harp traits::slice::test::test_slice
       START             harp utils::tests::test_r_str_to_utf8_replaces_invalid_utf8

running 1 test
test utils::tests::test_r_str_to_utf8_replaces_invalid_utf8 ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 88 filtered out; finished in 0.17s

        PASS [   0.176s] harp utils::tests::test_r_str_to_utf8_replaces_invalid_utf8
       START             harp vec_format::tests::test_vec_format_empty

running 1 test
test vec_format::tests::test_vec_format_empty ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 88 filtered out; finished in 0.17s

        PASS [   0.177s] harp vec_format::tests::test_vec_format_empty
       START             harp vec_format::tests::test_vec_format_methods

running 1 test
test vec_format::tests::test_vec_format_methods ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 88 filtered out; finished in 0.17s

        PASS [   0.178s] harp vec_format::tests::test_vec_format_methods
       START             harp vec_format::tests::test_vec_format_truncation

running 1 test
test vec_format::tests::test_vec_format_truncation ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 88 filtered out; finished in 0.17s

        PASS [   0.180s] harp vec_format::tests::test_vec_format_truncation
       START             harp vector::character_vector::test::test_character_vector

running 1 test
test vector::character_vector::test::test_character_vector ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 88 filtered out; finished in 0.17s

        PASS [   0.176s] harp vector::character_vector::test::test_character_vector
       START             harp vector::character_vector::test::test_create

running 1 test
test vector::character_vector::test::test_create ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 88 filtered out; finished in 0.17s

        PASS [   0.177s] harp vector::character_vector::test::test_create
       START             harp vector::formatted_vector::tests::test_na_not_quoted

running 1 test
test vector::formatted_vector::tests::test_na_not_quoted ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 88 filtered out; finished in 0.17s

        PASS [   0.177s] harp vector::formatted_vector::tests::test_na_not_quoted
       START             harp vector::formatted_vector::tests::test_unconforming_format_method

running 1 test
test vector::formatted_vector::tests::test_unconforming_format_method ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 88 filtered out; finished in 0.17s

        PASS [   0.180s] harp vector::formatted_vector::tests::test_unconforming_format_method
       START             harp vector::list::test::test_list

running 1 test
test vector::list::test::test_list ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 88 filtered out; finished in 0.17s

        PASS [   0.177s] harp vector::list::test::test_list
       START             stdext all::tests::test_all

running 1 test
test all::tests::test_all ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 6 filtered out; finished in 0.00s

        PASS [   0.006s] stdext all::tests::test_all
       START             stdext any::tests::test_any

running 1 test
test any::tests::test_any ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 6 filtered out; finished in 0.00s

        PASS [   0.005s] stdext any::tests::test_any
       START             stdext case::tests::test_case

running 1 test
test case::tests::test_case ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 6 filtered out; finished in 0.00s

        PASS [   0.005s] stdext case::tests::test_case
       START             stdext event::tests::test_signals

running 1 test
test event::tests::test_signals ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 6 filtered out; finished in 0.00s

        PASS [   0.005s] stdext event::tests::test_signals
       START             stdext join::tests::test_join

running 1 test
test join::tests::test_join ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 6 filtered out; finished in 0.00s

        PASS [   0.005s] stdext join::tests::test_join
       START             stdext push::tests::test_join

running 1 test
test push::tests::test_join ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 6 filtered out; finished in 0.00s

        PASS [   0.005s] stdext push::tests::test_join
       START             stdext tests::test_cstr

running 1 test
test tests::test_cstr ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 6 filtered out; finished in 0.00s

        PASS [   0.005s] stdext tests::test_cstr
────────────
     Summary [ 127.002s] 384 tests run: 366 passed, 18 failed, 0 skipped
       ABORT [   2.492s] ark lsp::completions::sources::composite::call::tests::test_completions_after_user_types_part_of_an_argument_name
           - with code 0xc0000409: The system detected an overrun of a stack-based buffer in this application. This overrun could potentially allow a malicious user to gain control of this application. (os error 1282)
       ABORT [   0.906s] ark lsp::completions::sources::composite::call::tests::test_session_arguments
           - with code 0xc0000409: The system detected an overrun of a stack-based buffer in this application. This overrun could potentially allow a malicious user to gain control of this application. (os error 1282)
       ABORT [   0.885s] ark lsp::completions::sources::composite::pipe::tests::test_find_pipe_root_finds_objects
           - with code 0xc0000409: The system detected an overrun of a stack-based buffer in this application. This overrun could potentially allow a malicious user to gain control of this application. (os error 1282)
       ABORT [   0.900s] ark lsp::completions::sources::composite::pipe::tests::test_find_pipe_root_works_with_native_and_magrittr
           - with code 0xc0000409: The system detected an overrun of a stack-based buffer in this application. This overrun could potentially allow a malicious user to gain control of this application. (os error 1282)
       ABORT [   0.890s] ark lsp::completions::sources::unique::extractor::tests::test_dollar_completions_on_nonexistent_object
           - with code 0xc0000409: The system detected an overrun of a stack-based buffer in this application. This overrun could potentially allow a malicious user to gain control of this application. (os error 1282)
       ABORT [   0.916s] ark lsp::completions::sources::unique::subset::tests::test_string_subset_completions
           - with code 0xc0000409: The system detected an overrun of a stack-based buffer in this application. This overrun could potentially allow a malicious user to gain control of this application. (os error 1282)
       ABORT [   1.406s] ark::kernel test_execute_request_browser_incomplete
           - with code 0xc0000409: The system detected an overrun of a stack-based buffer in this application. This overrun could potentially allow a malicious user to gain control of this application. (os error 1282)
       ABORT [   1.564s] ark::kernel test_execute_request_incomplete
           - with code 0xc0000409: The system detected an overrun of a stack-based buffer in this application. This overrun could potentially allow a malicious user to gain control of this application. (os error 1282)
       ABORT [   1.497s] ark::kernel test_execute_request_incomplete_multiple_lines
           - with code 0xc0000409: The system detected an overrun of a stack-based buffer in this application. This overrun could potentially allow a malicious user to gain control of this application. (os error 1282)
       ABORT [   1.470s] ark::kernel test_execute_request_single_line_buffer_overflow
           - with code 0xc0000409: The system detected an overrun of a stack-based buffer in this application. This overrun could potentially allow a malicious user to gain control of this application. (os error 1282)
       ABORT [   1.599s] ark::kernel test_stdin_single_line_buffer_overflow
           - with code 0xc0000409: The system detected an overrun of a stack-based buffer in this application. This overrun could potentially allow a malicious user to gain control of this application. (os error 1282)
       ABORT [   1.542s] ark::kernel-notebook test_notebook_execute_request_incomplete
           - with code 0xc0000409: The system detected an overrun of a stack-based buffer in this application. This overrun could potentially allow a malicious user to gain control of this application. (os error 1282)
       ABORT [   1.507s] ark::kernel-notebook test_notebook_execute_request_incomplete_multiple_lines
           - with code 0xc0000409: The system detected an overrun of a stack-based buffer in this application. This overrun could potentially allow a malicious user to gain control of this application. (os error 1282)
       ABORT [   0.540s] harp exec::tests::test_basic_function_error
           - with code 0xc0000409: The system detected an overrun of a stack-based buffer in this application. This overrun could potentially allow a malicious user to gain control of this application. (os error 1282)
       ABORT [   0.520s] harp exec::tests::test_r_unwrap
           - with code 0xc0000409: The system detected an overrun of a stack-based buffer in this application. This overrun could potentially allow a malicious user to gain control of this application. (os error 1282)
       ABORT [   0.560s] harp exec::tests::test_top_level_exec
           - with code 0xc0000409: The system detected an overrun of a stack-based buffer in this application. This overrun could potentially allow a malicious user to gain control of this application. (os error 1282)
       ABORT [   0.531s] harp exec::tests::test_try_catch_error
           - with code 0xc0000409: The system detected an overrun of a stack-based buffer in this application. This overrun could potentially allow a malicious user to gain control of this application. (os error 1282)
       ABORT [   0.522s] harp parse::tests::test_parse_status
           - with code 0xc0000409: The system detected an overrun of a stack-based buffer in this application. This overrun could potentially allow a malicious user to gain control of this application. (os error 1282)
error: test run failed
```

</details>

@lionel- and I believe this is an issue on Windows that has cropped up between Rust 1.83 and 1.84, in particular we think it is related to our `try_catch()` implementation and https://github.com/rust-lang/rust/pull/129582. It is failing all of a sudden because GitHub Actions just updated their Windows runner to 1.84

It is hypothesized that the `closure` that calls C level R code is now getting dropped when R longjmps, causing things inside that closure to now also get dropped, and possibly that is causing issues all of a sudden. It is quite hard to tell at the moment.

## @DavisVaughan at 2025-01-24T19:27:32Z

Of note, I updated my local Rust version to 1.84 on my Mac and cannot reproduce the failures. So it has something to do with 1.84 and probably the way longjmps work on Windows?

## @DavisVaughan at 2025-01-31T16:54:37Z

Look into https://github.com/rust-lang/rust/issues/123470 because the backtrace looks similar, are we getting some double panic scenario now that we could catch?

## @DavisVaughan at 2025-02-21T20:59:48Z

I have made some serious progress here

```rs
static MSG: Lazy<CString> = Lazy::new( || CString::new("ouch").unwrap());

pub fn top_level_exec2() -> harp::Result<()>
{
    extern "C" fn callback(_args: *mut c_void)
    {
        // let msg = CString::new("ouch").unwrap();
        unsafe { Rf_error(MSG.as_ptr()) };
    }

    unsafe { R_ToplevelExec(Some(callback), std::ptr::null_mut()) };

    Ok(())
}
```

```rs
    #[test]
    fn test_top_level_exec() {
        crate::r_task(|| {
            let _ = top_level_exec2();
        })
    }
```

With this minimal reprex, you do NOT get a panic when using `MSG`, but if you uncomment the `let msg = CString::new("ouch").unwrap()` line and swap `MSG` for `msg` then you DO get a panic.

I believe this means we are right-ish with our thinking that it has to do with Drop handling. The `Rf_error()` longjmps out of the Rust `callback` and that longjmp is correctly caught by `R_TopLevelExec()`, but the big scary important detail is that anything inside the `callback` itself either won't be Drop-ped or is being Drop-ped in such a way that it causes a panic - in this case that is the local `msg` `CString` causing the panic.

Still not sure why or how we fix it, but getting down to this reprex of crash vs no crash is big

## @DavisVaughan at 2025-02-21T21:33:26Z

Ooooh ok I don't understand the full implications of this yet but changing from `extern "C" fn callback` to `extern "C-unwind" fn callback` allows my example to pass. This also requires changing the libr binding of `R_ToplevelExec` to use `"C-unwind"`, but maybe there is something here. Adding it in a few other places allows the test suite to pass too.

## @lionel- at 2025-02-24T12:27:27Z

> but the big scary important detail is that anything inside the callback itself either won't be Drop-ped or is being Drop-ped in such a way that it causes a panic

That's right, this top-level-exec is only a safeguard. It's still important to run code in the safest way possible inside the callback.

## @lionel- at 2025-02-24T12:32:57Z

Do you think the crashes are due to the explicit leak cases we have in unit tests, e.g. here https://github.com/posit-dev/ark/blob/06153bac1b6196e22a58d641adeabb8b46336b64/crates/harp/src/exec.rs#L572?

If that's the case, I think the current situation is okayish and would be solved by #718?