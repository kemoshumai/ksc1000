
//! === AST ===
// ASTはコンパイラやトランスパイラに渡されるプログラムの情報です。
// 文字の羅列だったプログラムを木構造にしたものです。
// コンパイラはこの木構造をもとに各媒体へとコンパイルします。
// そのため、具体的な文法に関しての情報は含まれるべきではなく、プログラムの流れを含め構造の状態を扱うべきです。

/// プログラムは複数の文から構成される。
struct Program{
    statements: Vec<Statement>
}

/// 文
enum Statement{
    ExpressionStatement(ExpressionStatement)
}

/// 式文。式はセミコロンを付けると文になる。ブロックは例外的に式にも文にも成れず、式文にのみ成れる。
enum ExpressionStatement{
    Expression(Expression),
    Block(Block)
}

/// 式。評価できるもの。
enum Expression{
    FunctionCall(FunctionCall)
}

/// 関数呼び出し式
struct FunctionCall{
    callname: Identifier,
    args: Vec<Expression>
}

/// 識別子
struct Identifier{
    name: String
}

/// ブロック。複数の文をまとめる。
struct Block{
    statements: Vec<Statement>
}

///関数宣言文
struct FunctionDeclaration{
    name: Identifier,
    params: Vec<Param>,
    content: ExpressionStatement
}

///仮引数
struct Param{
    value_type: Identifier,
    name: Identifier
}

///変数宣言式
struct VariableDeclaration{
    value_type: Identifier,
    name: Identifier,
    init_value: Expression
}

///if式
struct IfExpression{
    condition: Expression,
    then_expression: ExpressionStatement,
    else_expression: ExpressionStatement
}

///for式
struct ForExpression{
    loop_as: Identifier,
    pump_from: Identifier,
    content: ExpressionStatement
}

///while式
struct WhileExpression{
    condition: Expression,
    contemt: ExpressionStatement
}

///固定文字列リテラル
struct StringLiteral{
    string: String
}

///固定文字列リテラル
struct NumberLiteral{
    number: f64
}


fn main() {
    println!("Hello, world!");
}
