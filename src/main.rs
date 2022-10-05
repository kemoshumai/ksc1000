
//! === AST ===
// ASTはコンパイラやトランスパイラに渡されるプログラムの情報です。
// 文字の羅列だったプログラムを木構造にしたものです。
// コンパイラはこの木構造をもとに各媒体へとコンパイルします。
// そのため、具体的な文法に関しての情報は含まれるべきではなく、プログラムの流れを含め構造の状態を扱うべきです。

use std::{collections::HashMap, env};

use inkwell::{values::{FloatValue, AnyValue, AnyValueEnum, PointerValue, BasicMetadataValueEnum, BasicValueEnum}, context::Context, builder::Builder, module::Module, support::LLVMString, FloatPredicate};
use log::{info, debug, error};

/// プログラムは複数の文から構成される。
struct Program{
    statements: Vec<Statement>
}

/// 文
enum Statement{
    ExpressionStatement(ExpressionStatement),
    FunctionDeclaration(FunctionDeclaration)
}

/// 式文。式はセミコロンを付けると文になる。ブロックは例外的に式にも文にも成れず、式文にのみ成れる。
enum ExpressionStatement{
    Expression(Expression),
    Block(Block)
}

/// 式。評価できるもの。
enum Expression{
    FunctionCall(FunctionCall),
    VariableDeclaration(Box<VariableDeclaration>),
    IfExpression(Box<IfExpression>),
    ForExpression(Box<ForExpression>),
    WhileExpression(Box<WhileExpression>),
    StringLiteral(StringLiteral),
    NumberLiteral(NumberLiteral),
    BinaryOperator(BinaryOperator),
    EvalVariableExpression(EvalVariableExpression)
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

///二項演算子
struct BinaryOperator{
    method: BinaryOperatorMethod,
    left: Box<Expression>,
    right: Box<Expression>
}

///二項演算子の種類
enum BinaryOperatorMethod{
    ADD, SUB, MUL, DIV, DivInt
}

///変数呼び出し
struct EvalVariableExpression{
    target: Identifier
}

///固定文字列リテラル
struct StringLiteral{
    string: String
}

///固定文字列リテラル
struct NumberLiteral{
    number: f64
}

// ! === Compiler LLVM ===
// ここからはコンパイラのコードです。
// Inkwellを使いASTをLLVM-IRに変換します。

/// 式が評価された結果返り得る値のInkwell表現
enum Object <'ctx>{
    Number(FloatValue<'ctx>),
    Undefined
}

/// コンパイラ
struct Compiler<'ctx>{
    context: &'ctx Context,
    builder: Builder<'ctx>,
    module: Module<'ctx>,
    variable_table: HashMap<String, PointerValue<'ctx>>
}

/// コンパイラのコンパイル機能
impl<'ctx> Compiler<'ctx>{

    /// ASTのルートから再帰的にコンパイルを実行
    /// 借用で渡されたProgram構造体をコンパイルし、LLVM-IRを返す。
    fn compile(&mut self, programroot:&Program, is_entry: bool) -> Result<String, std::str::Utf8Error>{
        debug!("Compiling init....");
        self.compile_main_function(is_entry);
        let Program { statements } = programroot;
        for statement in statements{
            self.compile_statement(statement);
        }
        return Ok(String::from(self.module.print_to_string().to_str()?));
    }

    /// メイン関数を作る
    fn compile_main_function(&self, is_entry: bool) {
        debug!("Auto-declaring entry main function.");

        // 作成
        let name = if is_entry {"main"} else {"initial"};
        let i32_type = self.context.i32_type();
        let fn_ty = i32_type.fn_type(&[], false);
        let main_func = self.module.add_function(name, fn_ty, None);

        // main関数へ操作対象を移動
        info!("Change current block to entry.");
        let name = if is_entry {"main"} else {"initial"};
        let basic_block = self.context.append_basic_block(main_func, name);
        self.builder.position_at_end(basic_block);
    }

    /// 文をコンパイルする
    fn compile_statement(&self, statement: &Statement) {
        let _ = match statement {
            Statement::ExpressionStatement(statement) => self.compile_expression_statement(&statement),
            Statement::FunctionDeclaration(_) => todo!(),
        };
    }

    /// 式文をコンパイルする
    fn compile_expression_statement(&self, expression_statement: &ExpressionStatement) -> Object{
        match expression_statement {
            ExpressionStatement::Block(block) => self.compile_block(block),
            ExpressionStatement::Expression(exp) => self.compile_expression(exp)
        }
    }

    /// ブロックをコンパイルする
    //TODO: if式などと組み合わせられるように
    fn compile_block(&self, block: &Block) -> Object{
        //ブロック文式は評価するとundeifnedを返す。
        return Object::Undefined;
    }

    /// 式をコンパイルする
    fn compile_expression(&self, expression: &Expression) -> Object{
        match expression {
            Expression::FunctionCall(functioncall) => self.compile_functioncall(functioncall),
            Expression::VariableDeclaration(_) => todo!(),
            Expression::IfExpression(if_expression) => self.compile_if_expression(if_expression),
            Expression::ForExpression(_) => todo!(),
            Expression::WhileExpression(_) => todo!(),
            Expression::StringLiteral(_) => todo!(),
            Expression::NumberLiteral(_) => todo!(),
            Expression::BinaryOperator(_) => todo!(),
            Expression::EvalVariableExpression(_) => todo!(),
        }
    }

    /// 関数呼び出し式をコンパイルする
    fn compile_functioncall(&self, functioncall: &FunctionCall) -> Object{
        debug!("Compiling function call");

        // 評価された結果を格納するベクトル
        let mut solved_args = Vec::with_capacity(functioncall.args.len());

        // 実引数を値へと評価
        for arg in &functioncall.args {
            if let Object::Number(solved) = self.compile_expression(&arg) {
                solved_args.push(solved);
            }
        }

        let argsv: Vec<BasicMetadataValueEnum> = solved_args.iter().by_ref().map(|&val| val.into()).collect();

        // 関数を名前から検索
        let function = self.module.get_function(&functioncall.callname.name).unwrap_or_else(|| {
            error!("Undefined function {} is called!", &functioncall.callname.name);
            panic!("A fatal error occured. Compiling is stopped.");
        });
        
        // BasicValueとして取得
        //TODO: 構造体などユーザー定義の型も許可したい
        let returned_value = self.builder.build_call(function, &argsv, "returned").try_as_basic_value();

        // BasicValueをObjectへ変換
        if let Some(returnable) = returned_value.left() {
            return match returnable {
                BasicValueEnum::FloatValue(v) => Object::Number(v),
                _ => Object::Undefined
            }
        } else {
            return Object::Undefined;
        }
    }


    fn compile_variable_declaration(&mut self, variable_decralation: &VariableDeclaration) -> Object{
        let ty = match variable_decralation.value_type.name.as_str() {
            "Number" => self.context.f64_type(),
            _ => panic!("Non-primitve type is not implemented.")
        };
        let var_pointer = self.builder.build_alloca(ty, &variable_decralation.name.name);
        if let Object::Number(basicvalue) = self.compile_expression(&variable_decralation.init_value) {
            // もし返り値がNumber型だったら
            self.builder.build_store(var_pointer, basicvalue);
        }
        self.variable_table.insert(variable_decralation.name.name.clone(), var_pointer);
        return Object::Number(var_pointer.as_any_value_enum().into_float_value());
    }

    fn compile_if_expression(&self, if_expression: &IfExpression) -> Object{
        let zero_const = self.context.f64_type().const_float(0.0);
        let condition_object = self.compile_expression(&if_expression.condition);
        return match condition_object {
            Object::Number(floatvalue) => {
                let condition = self
                    .builder
                    .build_float_compare(FloatPredicate::ONE, floatvalue, zero_const, "ifcond");


                //TODO: メイン関数以外でもif式を使えるようにする
                let parent = self.module.get_function("main").unwrap_or_else(|| panic!("You can use if expression only in main function!"));
                
                let then_bb = self.context.append_basic_block(parent, "then");
                let else_bb = self.context.append_basic_block(parent, "else");
                let cont_bb = self.context.append_basic_block(parent, "ifcont");
    
                self.builder.build_conditional_branch(condition, then_bb, else_bb);

                // build then block
                self.builder.position_at_end(then_bb);
                let then_val = self.compile_expression_statement(&if_expression.then_expression);
                self.builder.build_unconditional_branch(cont_bb);

                let then_bb = self.builder.get_insert_block().unwrap();

                // build else block
                self.builder.position_at_end(else_bb);
                let else_val = self.compile_expression_statement(&if_expression.else_expression);
                self.builder.build_unconditional_branch(cont_bb);

                let else_bb = self.builder.get_insert_block().unwrap();

                // emit merge block
                self.builder.position_at_end(cont_bb);

                let phi = self.builder.build_phi(self.context.f64_type(), "iftmp");

                if let Object::Number(then_val) = then_val {
                    if let Object::Number(else_val) = else_val {
                        phi.add_incoming(&[(&then_val, then_bb), (&else_val, else_bb)]);
                        return Object::Number(phi.as_basic_value().into_float_value());
                    }else{
                        return Object::Undefined;
                    }
                }else{
                    return Object::Undefined;
                }
            },
            Object::Undefined => Object::Undefined
        }
        
    }



}



fn main() {
    env::set_var("RUST_LOG", "debug");
    env_logger::init();
    
    let context = Context::create();
    let module = context.create_module("main");
    let builder = context.create_builder();
    let mut compiler = Compiler{
        context: &context,
        module,
        builder,
        variable_table: HashMap::new()
    };

    let program = Program{
        statements:vec![
            Statement::FunctionDeclaration(
                FunctionDeclaration{
                    name: Identifier{
                        name: String::from("pow")
                    },
                    params: vec![
                        Param{
                            value_type: Identifier{
                                name: String::from("Number")
                            },
                            name: Identifier{
                                name: String::from("n")
                            }
                        }
                    ],
                    content: ExpressionStatement::Block(Block{
                        statements: vec![
                            Statement::ExpressionStatement(
                                ExpressionStatement::Expression(
                                    Expression::BinaryOperator(BinaryOperator{
                                        method:BinaryOperatorMethod::ADD,
                                        left: Box::from(Expression::EvalVariableExpression(EvalVariableExpression { target: Identifier{name: String::from("n")} })),
                                        right: Box::from(Expression::EvalVariableExpression(EvalVariableExpression { target: Identifier{name: String::from("n")} }))
                                    })
                                )
                            )
                        ]
                    })
                }
            ),
            Statement::ExpressionStatement(
                ExpressionStatement::Expression(
                    Expression::VariableDeclaration(Box::from(VariableDeclaration{
                        value_type: Identifier { name: String::from("Number") },
                        name: Identifier { name: String::from("n") },
                        init_value: Expression::NumberLiteral(NumberLiteral { number: 2.0 })
                    }))
                )
            ),
            Statement::ExpressionStatement(
                ExpressionStatement::Expression(
                    Expression::FunctionCall(FunctionCall{
                        callname: Identifier{
                            name: String::from("print")
                        },
                        args: vec![
                            Expression::FunctionCall(FunctionCall {
                                callname: Identifier { name: String::from("pow") },
                                args: vec![Expression::EvalVariableExpression(EvalVariableExpression {
                                    target: Identifier { name: String::from("n") }
                                })]
                            })
                        ]
                    })
                )
            )
        ]
    };

    match compiler.compile(&program, true) 
    {
        Ok(compiled_ir) => {
            println!("======== LLVM IR ========");
            println!("{}",compiled_ir);
            println!("========== END ==========");
        },
        Err(e) => panic!("Compile error! {}", e),
    }
}
