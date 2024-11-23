// You can import anything defined in the dependencies table of the crate.
use ui_test::Config;

fn wrong_type() {
    let _ = Config::this_function_does_not_exist();
    //~^ E0599
}
