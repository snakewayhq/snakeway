pub mod args;
mod solve;

pub use args::RouteCmd;

pub fn run(cmd: RouteCmd) {
    match cmd {
        RouteCmd::Solve(args) => solve::run(args),
    }
}
