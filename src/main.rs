use anyhow::Result;

mod dot;
mod pert;

use dot::PertDot;
use pert::DataLoader;

fn main() -> Result<()> {
    //  CSV file example (start, end, task duration, task name)
    //   1, 2, 1, task1
    //   2, 3, 3, task2
    //   1, 3, 5, task3
    //   1, 4, 10, task4
    //   3, 4, 2, task5

    let data_loader = DataLoader::from_stdin()?;
    let pert = data_loader.to_graph()?;

    println!("{}", PertDot::new(&pert.0));
    Ok(())
}
