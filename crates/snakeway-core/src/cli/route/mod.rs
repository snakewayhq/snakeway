pub(crate) mod args;
pub mod solve;
pub(crate) use args::RouteCmd;

pub(crate) fn run(cmd: RouteCmd) {
    match cmd {
        RouteCmd::Solve(args) => solve::run(args),
    }
}
