CARGO_COMMAND = cargo
CLIPPY_PARAMS = -- -W clippy::cargo -W clippy::nursery -W clippy::expect_used -W clippy::unwrap_used -W clippy::items_after_statements -W clippy::if_not_else -W clippy::trivially_copy_pass_by_ref -W clippy::match_same_arms -D warnings
ifeq ($(CLIPPY_PEDANTIC),1)
  CLIPPY_PARAMS += -W clippy::pedantic
endif

check:
	$(CARGO_COMMAND) fmt --all
	$(CARGO_COMMAND) clippy --tests $(CLIPPY_PARAMS)
	$(CARGO_COMMAND) clippy --lib --bins $(CLIPPY_PARAMS) -D clippy::panic
	$(CARGO_COMMAND) test
