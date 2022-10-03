use pest::{self, iterators::Pair, Parser};

#[derive(pest_derive::Parser)]
#[grammar = "grammer.pest"]
struct ProgramParser;


fn plot(pair: Pair<Rule>, indent: i32) {
    for _ in 0..indent { print!("  ") }
    if indent != 0 {
        print!("-> ");
    }
    let span = pair.as_span();
    println!("{:?} \x1b[36m({:?})\x1b[0m [{}..{}]", pair.as_rule(), span.as_str(), span.start(), span.end());
    for i in pair.into_inner(){
        plot(i, indent+1);
    }
}



fn main() {
    let sourcecode = " \
        Number a = 42;
        {
            Number b = a;
            Number c = b*4;
        }
        Number d = 1 + 2 * 3;
        みーしぇちゃんかわいいね
    ";
    let mut pairs = ProgramParser::parse(Rule::Program, sourcecode).unwrap();
    plot(pairs.next().unwrap(), 0);
}
