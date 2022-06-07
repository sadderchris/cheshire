#[derive(Debug, Parser)]
#[grammar = "scanner/grammar/grammar.pest"]
pub struct SchemeParser;

#[cfg(test)]
mod test;
