//! turn the model embedded in the macros into a class diagram

use super::{Result, create};
use std::io::Write as _;
use std::path::PathBuf;

/// write the model to the specified output (dash means stdout)
pub fn run(output: &PathBuf) -> Result<()> {
    let mut stream = create(output)?;
    writeln!(stream, "classDiagram")?;
    tindalwic_macros::template! { ? class ^
        writeln!(stream, "class {} <<{}>> {{", #CLASS, #CATEGORY)?;
        let vis = if #CATEGORY== "opaque" {"-"} else {"+"};
        { ^
            writeln!(stream,
                ( ? array ^ "  {}&[{}] {}" )
                ( ? option ^ "  {}Some[{}] {}\n  +None" )
                ( ? else ^ "  {}{} {}" )
                , vis, #LINK, #NAME )?;
        }
        writeln!(stream, "}}")?;
    }
    tindalwic_macros::template! { ? struct enum array ^
        if false { ^ ( ? Comment ^ || true ) } { { ^
            ( ? Text ^ writeln!(stream, "{} {} Comment", #CLASS, "..()")?; )
            ( ? List ^ writeln!(stream, "{} {} Comment", #CLASS, "..()")?; )
            ( ? else ^ writeln!(stream, "{} {} Comment", #CLASS, "..")?; )
        } }
        { ^
            ( ? primitive ^ ) ( ? Comment ^ )
            ( ? else ^ writeln!(stream, "{} *-- {}", #CLASS, #LINK)?; )
        }
    }
    writeln!(stream, "Comment *-- Lines\n` ` ().. Comment")?;
    Ok(())
}
