use std::fmt::{self, Display, Write};

use super::pert::{Event, PertGraph, Task};

pub struct PertDot<'a> {
    graph: &'a PertGraph,
}

static INDENT: &str = "    ";

impl<'a> PertDot<'a> {
    /// Create a `Dot` formatting wrapper with default configuration.
    pub fn new(graph: &'a PertGraph) -> Self {
        PertDot { graph }
    }
}

use petgraph::visit::{EdgeRef, IntoNodeReferences, NodeIndexable};

impl<'a> PertDot<'a> {
    fn graph_fmt<NF, EF>(
        &self,
        g: &'a PertGraph,
        f: &mut fmt::Formatter,
        mut node_fmt: NF,
        mut edge_fmt: EF,
    ) -> fmt::Result
    where
        NF: FnMut(&Event, &mut dyn FnMut(&dyn Display) -> fmt::Result) -> fmt::Result,
        EF: FnMut(&Task, &mut dyn FnMut(&dyn Display) -> fmt::Result) -> fmt::Result,
    {
        writeln!(f, "digraph PERT {{\n{}graph [rankdir = \"LR\"];", INDENT)?;

        // output all events
        for (node, event) in g.node_references() {
            write!(f, "{}{}", INDENT, g.to_index(node))?;
            write!(f, " [label=\"")?;
            node_fmt(event, &mut |d| Escaped(d).fmt(f))?;
            writeln!(f, "\"]")?;
        }
        // output all edges
        for edge in g.edge_references() {
            write!(
                f,
                "{}{} -> {}",
                INDENT,
                g.to_index(edge.source()),
                g.to_index(edge.target())
            )?;

            write!(f, " [label=\"")?;
            edge_fmt(edge.weight(), &mut |d| Escaped(d).fmt(f))?;
            write!(f, "\"")?;
            match edge.weight() {
                t if t.is_dummy_path() => write!(f, ", style=dashed")?,
                t if t.is_critical_path() => write!(f, ", style=bold")?,
                _ => {}
            };
            writeln!(f, "]")?;
        }

        writeln!(f, "}}")?;
        Ok(())
    }
}

impl<'a> fmt::Display for PertDot<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.graph_fmt(self.graph, f, |n, cb| cb(n), |e, cb| cb(e))
    }
}

impl<'a> fmt::Debug for PertDot<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.graph_fmt(
            self.graph,
            f,
            |n, cb| cb(&DebugFmt(n)),
            |e, cb| cb(&DebugFmt(e)),
        )
    }
}

/// Escape for Graphviz
struct Escaper<W>(W);

impl<W> fmt::Write for Escaper<W>
where
    W: fmt::Write,
{
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            self.write_char(c)?;
        }
        Ok(())
    }

    fn write_char(&mut self, c: char) -> fmt::Result {
        match c {
            '"' | '\\' => self.0.write_char('\\')?,
            // \l is for left justified linebreak
            '\n' => return self.0.write_str("\\l"),
            _ => {}
        }
        self.0.write_char(c)
    }
}

/// Pass Display formatting through a simple escaping filter
struct Escaped<T>(T);

impl<T> fmt::Display for Escaped<T>
where
    T: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if f.alternate() {
            Ok(writeln!(&mut Escaper(f), "{:#}", &self.0)?)
        } else {
            Ok(write!(&mut Escaper(f), "{}", &self.0)?)
        }
    }
}

/// Pass Debug formatting to Display
struct DebugFmt<T>(T);

impl<T> fmt::Display for DebugFmt<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}
