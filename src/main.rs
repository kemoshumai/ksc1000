use crypto::{sha2::Sha256, digest::Digest};
use inkwell::{context::Context, builder::Builder, module::Module, types::{AnyTypeEnum, BasicMetadataTypeEnum, BasicTypeEnum, PointerType, FunctionType, AnyType}, values::{FunctionValue, BasicValue, AnyValue, BasicValueEnum, IntValue, AnyValueEnum, PointerValue, BasicMetadataValueEnum}, IntPredicate, basic_block::BasicBlock, FloatPredicate, AddressSpace};
use std::{env, collections::HashMap, mem::discriminant, path::PathBuf};
use std::fs::File;
use std::io::prelude::*;
use uuid::Uuid;

enum Predicate{
    EQUAL,
    NOT_EQUAL,
    GREATER_THAN,
    GREATER_THAN_OR_EQUAL,
    LESS_THAN,
    LESS_THAN_OR_EQUAL,
}

enum BinaryOperator{
    ADD,SUB,MUL,DIV,
    REM
}

struct KSCType<'ctx>{
    name: String,
    reference: AnyTypeEnum<'ctx>
}

struct KSCValue<'ctx>{
    valuetype: KSCType<'ctx>,
    value: AnyValueEnum<'ctx>
}

/// スタック(スコープごとに用意する、定義された変数や型を保存するもの。スコープを抜けるとpop)
struct Stack<'ctx>{
    types: Vec<KSCType<'ctx>>,
    values: Vec<KSCValue<'ctx>>
}

/// コンパイラ構造体
struct Compiler<'a, 'ctx>{
    context: &'ctx Context,
    builder: &'a Builder<'ctx>,
    module: Option<Module<'ctx>>,
    stack_function: Vec<&'a str>,
    stack: Vec<Stack<'ctx>>
}

/// スタックなど変数や型の管理のための関連関数
impl<'a, 'ctx> Compiler<'a, 'ctx>{

    /// 新しい型を最新のスタックに登録
    fn insert_new_type_to_stack(&mut self, ksctype: KSCType<'ctx>) {
        self.stack.last_mut()
            .unwrap_or_else(||panic!("There is no stack yet!"))
            .types
            .push(ksctype);
    }

    /// 型名からKSCTypeを検索して返す
    fn get_ksctype_from_name(&self, name: &'a str) -> Option<&KSCType<'ctx>> where 'a: 'ctx{
        for scope in &self.stack{
            let found = scope.types.iter().find(|ksct| ksct.name == name);
            match found{
                Some(_) => return found,
                None => continue,
            }
        }
        return None;
    }
}

/// コンパイル関連関数 (実際にIRを書く)
impl<'a, 'ctx> Compiler<'a, 'ctx> where 'a: 'ctx{

    fn new (context: &'a Context, builder: &'a Builder) -> Compiler<'a, 'ctx>{
        return Compiler{
            context,
            builder,
            module: None,
            stack_function: vec![],
            stack: vec![Stack{ types: vec![], values: vec![] }]
        };
    }

    fn emit_as_text(&self) -> Option<String> {
        if let Some(module) = &self.module {
            return Some(module.print_to_string().to_string());
        }
        return None;
    }

    /// 関数型を取得
    fn get_function_type(&self, return_type: &'a AnyTypeEnum) -> FunctionType<'ctx> {
        return return_type.into_function_type();
    }

    /// プリミティブ型を定義
    fn init_primitive_types(&mut self) {

        // Number -> f64
        self.insert_new_type_to_stack(
            KSCType {
                name: String::from("Number"),
                reference: AnyTypeEnum::FloatType(self.context.f64_type())
            }
        );

        // i32 -> i32
        self.insert_new_type_to_stack(
            KSCType {
                name: String::from("i32"),
                reference: AnyTypeEnum::IntType(self.context.i32_type())
            }
        );
        
        // bool -> i1
        self.insert_new_type_to_stack(
            KSCType {
                name: String::from("bool"),
                reference: AnyTypeEnum::IntType(self.context.custom_width_int_type(1))
            }
        );
        
        // void -> void
        self.insert_new_type_to_stack(
            KSCType {
                name: String::from("void"),
                reference: AnyTypeEnum::VoidType(self.context.void_type())
            }
        );

    }

    /// モジュールを作成
    fn create_module(&mut self, module_name: &str) {
        self.module = Some(self.context.create_module(module_name));
    }

    /// 関数を作成(宣言してブロックを作成)
    fn create_function(&mut self, name: &'a str, return_type: &'a str, param_types: &Vec<&'a str>, param_names: &Vec<&'a str>) -> FunctionValue<'ctx> {
        self.stack_function.push(name);

        // 仮引数の型を参照
        let param_types = &param_types.iter().map(|param_type| {
            if let Some(param_type) = self.get_ksctype_from_name(param_type) {
                return match param_type.reference {
                    AnyTypeEnum::ArrayType(t) => BasicMetadataTypeEnum::ArrayType(t),
                    AnyTypeEnum::FloatType(t) => BasicMetadataTypeEnum::FloatType(t),
                    AnyTypeEnum::FunctionType(_) => panic!("Function type cannot be param."),
                    AnyTypeEnum::IntType(t) => BasicMetadataTypeEnum::IntType(t),
                    AnyTypeEnum::PointerType(t) => BasicMetadataTypeEnum::PointerType(t),
                    AnyTypeEnum::StructType(t) => BasicMetadataTypeEnum::StructType(t),
                    AnyTypeEnum::VectorType(t) => BasicMetadataTypeEnum::VectorType(t),
                    AnyTypeEnum::VoidType(_) => panic!("Void type cannot be param."),
                }
            } else {
                panic!("Param type ({}) not defined!", param_type);
            }
        }).collect::<Vec<BasicMetadataTypeEnum>>();

        // 戻り値の型を参照
        if let Some(return_type) = self.get_ksctype_from_name(&return_type) {

            let fn_type = match return_type.reference {
                AnyTypeEnum::ArrayType(t) => t.fn_type(param_types, false),
                AnyTypeEnum::FloatType(t) => t.fn_type(param_types, false),
                AnyTypeEnum::FunctionType(_) => panic!("Function type cannot be returned."),
                AnyTypeEnum::IntType(t) => t.fn_type(param_types, false),
                AnyTypeEnum::PointerType(t) => t.fn_type(param_types, false),
                AnyTypeEnum::StructType(t) => t.fn_type(param_types, false),
                AnyTypeEnum::VectorType(t) => t.fn_type(param_types, false),
                AnyTypeEnum::VoidType(t) => t.fn_type(param_types, false),
            };
            if let Some(module) = &self.module {
                let func = module.add_function(name, fn_type, None);
                let func_bb = self.context.append_basic_block(func, name);
                self.builder.position_at_end(func_bb);
                if param_types.len() != param_names.len() {
                    panic!("The number of parameters does not match the type and name.");
                }
                for (i, arg) in func.get_param_iter().enumerate() {
                    let param_name = param_names[i];
                    let alloca = self.builder.build_alloca(arg.get_type(), param_name);
                    self.builder.build_store(alloca, arg);
                }
                return func;
            }
            else
            {
                panic!("Failed to create function ({}). There is no Module yet. Create module first.", name);
            }
        }
        else {
            panic!("Return type ({}) not defined!", return_type);
        }
    }

    /// 関数を作成(宣言のみ)
    fn create_function_declare(&mut self, name: &'a str, return_type: &AnyTypeEnum<'ctx>, param_types: &Vec<AnyTypeEnum<'ctx>>) -> FunctionValue<'ctx> {

        // 仮引数の型を参照
        let param_types = &param_types.iter().map(|param_type| {
            return match param_type {
                AnyTypeEnum::ArrayType(t) => BasicMetadataTypeEnum::ArrayType(*t),
                AnyTypeEnum::FloatType(t) => BasicMetadataTypeEnum::FloatType(*t),
                AnyTypeEnum::FunctionType(_) => panic!("Function type cannot be param."),
                AnyTypeEnum::IntType(t) => BasicMetadataTypeEnum::IntType(*t),
                AnyTypeEnum::PointerType(t) => BasicMetadataTypeEnum::PointerType(*t),
                AnyTypeEnum::StructType(t) => BasicMetadataTypeEnum::StructType(*t),
                AnyTypeEnum::VectorType(t) => BasicMetadataTypeEnum::VectorType(*t),
                AnyTypeEnum::VoidType(_) => panic!("Void type cannot be param."),
            }
        }).collect::<Vec<BasicMetadataTypeEnum>>();

        // 戻り値の型を参照
        let fn_type = match return_type {
            AnyTypeEnum::ArrayType(t) => t.fn_type(param_types, false),
            AnyTypeEnum::FloatType(t) => t.fn_type(param_types, false),
            AnyTypeEnum::FunctionType(_) => panic!("Function type cannot be returned."),
            AnyTypeEnum::IntType(t) => t.fn_type(param_types, false),
            AnyTypeEnum::PointerType(t) => t.fn_type(param_types, false),
            AnyTypeEnum::StructType(t) => t.fn_type(param_types, false),
            AnyTypeEnum::VectorType(t) => t.fn_type(param_types, false),
            AnyTypeEnum::VoidType(t) => t.fn_type(param_types, false),
        };
        if let Some(module) = &self.module {
            self.stack_function.push(name);
            return module.add_function(name, fn_type, None);
        }
        else
        {
            panic!("Failed to craete function ({}). There is no Module yet. Create module first.", name);
        }

    }

    /// return文を作成
    fn create_return(&self, value: &Option<BasicValueEnum>) {
        if let Some(value) = value {
            self.builder.build_return(Some(value));
        } else{
            self.builder.build_return(None);
        }
    }

    /// if式を作成(分岐側)
    /// (condition_bool) ? (then_value) : (else_value)
    fn create_if_branch(&self, condition_bool: IntValue) -> (BasicBlock<'ctx>, BasicBlock<'ctx>, BasicBlock<'ctx>) {
        let zero_const = self.context.custom_width_int_type(1).const_zero();
        let condition = self
                    .builder
                    .build_int_compare(IntPredicate::NE, condition_bool, zero_const, "ifcond");
        
        let parent_func_name = self.stack_function.last().unwrap_or_else(||panic!("No function found!"));
        let parent = self.module.as_ref()
                        .unwrap_or_else(||panic!("No module."))
                        .get_function(&parent_func_name)
                        .unwrap_or_else(||panic!("No function."));

        let then_block = self.context.append_basic_block(parent, "then");
        let else_block = self.context.append_basic_block(parent, "else");
        let cont_block = self.context.append_basic_block(parent, "ifcont");

        self.builder.build_conditional_branch(condition, then_block, else_block);

        return (then_block, else_block, cont_block);
    }

    /// if式を作成(書き込み対象のブロックを選ぶ)
    fn start_if_branch(&self, branch: &BasicBlock){
        self.builder.position_at_end(*branch);
    }

    /// if式を作成(書き込み終わり)
    fn end_if_branch(&self, branch: &BasicBlock) -> BasicBlock<'ctx>{
        self.builder.build_unconditional_branch(*branch);
        return self.builder.get_insert_block().unwrap();
    }

    /// if式を作成(マージ)
    fn merge_if_branch(&self, then_value: &BasicValueEnum, else_value: &BasicValueEnum, then_block: BasicBlock, else_block: BasicBlock, cont_block: BasicBlock, typename:&'a str) -> BasicValueEnum<'ctx>{
        self.builder.position_at_end(cont_block);
        if discriminant(then_value) != discriminant(else_value) {
            panic!("The return value on then and the return value on else have different types.");
        }
        if let Some(rettype) = self.get_ksctype_from_name(typename) {
            let rettype = match rettype.reference {
                AnyTypeEnum::ArrayType(t) => BasicTypeEnum::ArrayType(t),
                AnyTypeEnum::FloatType(t) => BasicTypeEnum::FloatType(t),
                AnyTypeEnum::FunctionType(_) => panic!("Function type cannot be param."),
                AnyTypeEnum::IntType(t) => BasicTypeEnum::IntType(t),
                AnyTypeEnum::PointerType(t) => BasicTypeEnum::PointerType(t),
                AnyTypeEnum::StructType(t) => BasicTypeEnum::StructType(t),
                AnyTypeEnum::VectorType(t) => BasicTypeEnum::VectorType(t),
                AnyTypeEnum::VoidType(_) => panic!("Void type cannot be param."),
            };
            let phi = self.builder.build_phi(rettype, "iftmp");
            phi.add_incoming(&[(then_value, then_block), (else_value, else_block)]);
            return phi.as_basic_value();
        }else{
            panic!("")
        }
    }

    /// 比較演算子
    fn create_comparison_operator(&self, op:Predicate ,left: BasicValueEnum, right: BasicValueEnum) -> IntValue<'ctx> {
        if discriminant(&left) != discriminant(&right) {
            panic!("The left value and the right value have different types.");
        }
        let condition = match left {
            BasicValueEnum::ArrayValue(_) => panic!("ArrayValue is not comparable."),
            BasicValueEnum::IntValue(_) => {
                let op = match op {
                    Predicate::EQUAL => IntPredicate::EQ,
                    Predicate::NOT_EQUAL => IntPredicate::NE,
                    Predicate::GREATER_THAN => IntPredicate::SGT,
                    Predicate::GREATER_THAN_OR_EQUAL => IntPredicate::SGE,
                    Predicate::LESS_THAN => IntPredicate::SLT,
                    Predicate::LESS_THAN_OR_EQUAL => IntPredicate::SLE,
                };
                if let (BasicValueEnum::IntValue(left), BasicValueEnum::IntValue(right)) = (left,right) {
                    self.builder.build_int_compare(op, left, right, "compared")
                } else{
                    panic!("The left value and the right value have different types.")
                }
            },
            BasicValueEnum::FloatValue(_) => {
                let op = match op {
                    Predicate::EQUAL => FloatPredicate::OEQ,
                    Predicate::NOT_EQUAL => FloatPredicate::ONE,
                    Predicate::GREATER_THAN => FloatPredicate::OGT,
                    Predicate::GREATER_THAN_OR_EQUAL => FloatPredicate::OGE,
                    Predicate::LESS_THAN => FloatPredicate::OLT,
                    Predicate::LESS_THAN_OR_EQUAL => FloatPredicate::OLE,
                };
                if let (BasicValueEnum::FloatValue(left), BasicValueEnum::FloatValue(right)) = (left,right) {
                    self.builder.build_float_compare(op, left, right, "compared")
                } else{
                    panic!("The left value and the right value have different types.")
                }
            },
            BasicValueEnum::PointerValue(_) => panic!("PointerValue is not comparable."),
            BasicValueEnum::StructValue(_) => panic!("StructValue is not comparable."),
            BasicValueEnum::VectorValue(_) => panic!("VectorValue is not comparable."),
        };
        let pointer = self.builder.build_alloca(self.context.custom_width_int_type(1), "compared_val");
        self.builder.build_store(pointer, condition);
        if let BasicValueEnum::IntValue(v) = self.builder.build_load(pointer,"") {
            return v;
        }else{panic!("Could not assign the comparison result to a variable with the correct type.")}
    }

    /// 定数
    /// TODO: 符号がマイナスな整数にも対応
    fn create_constant_number(&'ctx self,type_name: &'a str, number: f64) -> BasicValueEnum<'ctx> {
        if let Some(constant_type) = self.get_ksctype_from_name(type_name) {
            return match constant_type.reference {
                AnyTypeEnum::ArrayType(_) => panic!("Constants of type ArrayType cannot be declared!"),
                AnyTypeEnum::FloatType(floattype) => BasicValueEnum::FloatValue(floattype.const_float(number)),
                AnyTypeEnum::FunctionType(_) => panic!("Constants of type ArrayType cannot be declared!"),
                AnyTypeEnum::IntType(inttype) => BasicValueEnum::IntValue(inttype.const_int(number.round() as u64,false)),
                AnyTypeEnum::PointerType(_) => panic!("Constants of type ArrayType cannot be declared!"),
                AnyTypeEnum::StructType(_) => panic!("Constants of type ArrayType cannot be declared!"),
                AnyTypeEnum::VectorType(_) => panic!("Constants of type ArrayType cannot be declared!"),
                AnyTypeEnum::VoidType(_) => panic!("Constants of type ArrayType cannot be declared!"),
            }
        } else {
            panic!("Param type ({}) not defined!", type_name);
        }
    }

    /// 二項演算子
    fn create_binnary_operator(&self, op: BinaryOperator, left: &'a BasicValueEnum, right: &'a BasicValueEnum) -> BasicValueEnum<'ctx>{
        if discriminant(left) != discriminant(right) {
            panic!("The left value and the right value have different types.");
        }
        let ret:BasicValueEnum = match left{
            BasicValueEnum::ArrayValue(_) => panic!("Four arithmetic operations are not possible with ArrayValue."),
            BasicValueEnum::IntValue(left) => {
                if let BasicValueEnum::IntValue(right) = right {
                    BasicValueEnum::IntValue( match op {
                        BinaryOperator::ADD => self.builder.build_int_add(*left, *right, "add"),
                        BinaryOperator::SUB => self.builder.build_int_sub(*left, *right, "sub"),
                        BinaryOperator::MUL => self.builder.build_int_mul(*left, *right, "mul"),
                        BinaryOperator::DIV => self.builder.build_int_signed_div(*left, *right, "div"),
                        BinaryOperator::REM => self.builder.build_int_signed_rem(*left, *right, "rem"),
                    } )
                }else{
                    panic!("The left value and the right value have different types.");
                }
            },
            BasicValueEnum::FloatValue(left) => {
                if let BasicValueEnum::FloatValue(right) = right {
                    BasicValueEnum::FloatValue( match op {
                        BinaryOperator::ADD => self.builder.build_float_add(*left, *right, "add"),
                        BinaryOperator::SUB => self.builder.build_float_sub(*left, *right, "sub"),
                        BinaryOperator::MUL => self.builder.build_float_mul(*left, *right, "mul"),
                        BinaryOperator::DIV => self.builder.build_float_div(*left, *right, "div"),
                        BinaryOperator::REM => self.builder.build_float_rem(*left, *right, "rem"),
                    } )
                }else{
                    panic!("The left value and the right value have different types.");
                }
            },
            BasicValueEnum::PointerValue(_) => panic!("Four arithmetic operations are not possible with PointerValue."),
            BasicValueEnum::StructValue(_) => panic!("Four arithmetic operations are not possible with StructValue."),
            BasicValueEnum::VectorValue(_) => panic!("Four arithmetic operations are not possible with VectorValue."),
        };
        return ret;
    }


    /// 関数呼び出し
    fn create_function_call(&self, name: &str, args: &'a Vec<BasicValueEnum>) -> Option<BasicValueEnum<'ctx>>{
        if self.stack_function.contains(&name) == false{
            panic!("Function {} not found!", name);
        }
        if let Some(module) = &self.module {
            let func = module.get_function(name).unwrap_or_else(||panic!("Function {} not found!", name));
            let argsv: Vec<BasicMetadataValueEnum> = args.iter().by_ref().map(|&val| val.into()).collect();
            return self.builder.build_call(func, &argsv, name).try_as_basic_value().left();
        }else{
            panic!("There is no Module yet. Create module first.");
        }
    }
}


///式
enum Expression{
    ///関数
    Function{
        name: String,
        return_type: String,
        param_types: Vec<String>,
        param_names: Vec<String>,
        content: Vec<Expression>
    },

    ///変数宣言
    VariableDeclaration{
        typename: String,
        name: String,
        mutable: bool,
        value: Box<Expression>
    }
}


/// 意味解析関連関数 (ASTを解析して対応する関連関数にIRを書かせる)
impl<'a, 'ctx> Compiler<'a, 'ctx> where 'a: 'ctx{

    /// ファイルパスから実際のモジュール名を割り出してモジュールを作成する。
    fn initialize_module_by_filepath(&mut self, filepath: &PathBuf) {
        let filename = filepath.file_name().unwrap().to_string_lossy().to_string();
        let filepath_as_str = filepath.to_str().unwrap();
        let mut haser = Sha256::new();
        haser.input_str(filepath_as_str);
        let hex = haser.result_str();
        self.create_module((filename + &hex).as_str());
    }

    /// ASTを意味解析してLLVMを書く
    fn build(&mut self, program: &'a Vec<Expression>) where 'a: 'ctx{
        self.init_primitive_types();
        // self.create_function_declare("printNumber", "void", &vec!["Number"]);

        for expression in program{
            self.compile_expression(&expression);
        }
    }


    /// 式をコンパイルする
    fn compile_expression(&mut self, expression: &'ctx Expression) -> KSCValue<'ctx> where 'a: 'ctx{
        match expression {
            Expression::Function { name, return_type, param_types, param_names, content } => {

                // 適当な関数名をつける
                let param_types: Vec<&str> = param_types.iter().map(|s| &**s).collect();
                let param_names: Vec<&str> = param_names.iter().map(|s| &**s).collect();
                let func = self.create_function(name.as_str(), return_type.as_str(), &param_types, &param_names);
                let func_kscvalue = KSCValue{
                    valuetype: KSCType { name: "Function".to_string(), reference: func.get_type().as_any_type_enum() },
                    value: func.as_any_value_enum(),
                };
                return func_kscvalue;
            },
            Expression::VariableDeclaration { typename, name, mutable, value } => {
                let executed = self.compile_expression( &*value );
                let vartype = if executed.valuetype.name == "Function" {
                    &executed.valuetype
                } else {
                    self.get_ksctype_from_name(typename.as_str())
                                    .unwrap_or_else(||panic!("Type '{typename}' is not found!'"))
                };
                if vartype.name != executed.valuetype.name {
                    panic!("Cannot be assigned because the type is different. '{}' <= {}", vartype.name, executed.valuetype.name);
                }
                return executed;
            },
        }
    }
}


fn main() {
    env::set_var("RUST_LOG", "debug");
    env_logger::init();

    let program = vec![
        Expression::VariableDeclaration {
            typename: "Function".to_string(),
            name: "gcd".to_string(),
            mutable: false,
            value: Box::from(Expression::Function {
                name: "gcd".to_string(),
                return_type: "Number".to_string(),
                param_types: vec![
                    "Number".to_string(),
                    "Number".to_string()
                ],
                param_names: vec![
                    "a".to_string(),
                    "b".to_string()
                ],
                content: vec![]
            })
        }
    ];

    let context = Context::create();// 'ctx
    let builder = context.create_builder();
    let mut compiler = Compiler::new(&context,&builder);

    compiler.initialize_module_by_filepath(&PathBuf::from("./example.ksc"));
    
    compiler.build(&program);

    println!("======== LLVM IR ========");
    println!("{}", compiler.emit_as_text().unwrap());
    println!("========== END ==========");
    println!("{:?}", compiler.emit_as_text().unwrap());

    let filename = "./compiled/ksc.ll";
    let mut file = File::create(filename).unwrap();
    file.write_all(compiler.emit_as_text().unwrap().as_bytes()).unwrap();
}