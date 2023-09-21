CARGO_COMMAND = cargo
CLIPPY_PARAMS = -- \
	-W clippy::cargo \
	-W clippy::cast_lossless \
	-W clippy::dbg_macro \
	-W clippy::expect_used \
	-W clippy::if_not_else \
	-W clippy::items_after_statements \
	-W clippy::large_stack_arrays \
	-W clippy::linkedlist \
	-W clippy::manual_filter_map \
	-W clippy::match_same_arms \
	-W clippy::needless_continue \
	-W clippy::needless_pass_by_value \
	-W clippy::nursery \
	-W clippy::option_if_let_else \
	-W clippy::print_stderr \
	-W clippy::print_stdout \
	-W clippy::redundant_closure_for_method_calls \
	-W clippy::semicolon_if_nothing_returned \
	-W clippy::similar_names \
	-W clippy::single_match_else \
	-W clippy::trivially_copy_pass_by_ref \
	-W clippy::unnested_or_patterns \
	-W clippy::unreadable-literal \
	-W clippy::unseparated-literal-suffix \
	-D warnings
ifeq ($(CLIPPY_PEDANTIC),1)
  CLIPPY_PARAMS += -W clippy::pedantic
endif

check:
	$(CARGO_COMMAND) fmt --all
	$(CARGO_COMMAND) clippy --tests $(CLIPPY_PARAMS)
	$(CARGO_COMMAND) clippy --lib --bins $(CLIPPY_PARAMS) -D clippy::panic
	$(CARGO_COMMAND) test
	$(CARGO_COMMAND) test --release
