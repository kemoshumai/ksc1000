use inkwell::{context::Context, builder::Builder, module::Module, types::{AnyTypeEnum, BasicMetadataTypeEnum, BasicTypeEnum}, values::{FunctionValue, BasicValue, AnyValue, BasicValueEnum, IntValue, AnyValueEnum, PointerValue, BasicMetadataValueEnum}, IntPredicate, basic_block::BasicBlock, FloatPredicate};
use std::{env, collections::HashMap, mem::discriminant};

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

/// コンパイラ構造体
struct Compiler<'a, 'ctx>{
    context: &'ctx Context,
    builder: &'a Builder<'ctx>,
    module: Option<Module<'ctx>>,
    types: HashMap<String, AnyTypeEnum<'ctx>>,
    stack_function: Vec<&'a str>,
    stack: HashMap<&'a str, PointerValue<'ctx>>
}

/// コンパイル関連関数 (実際にIRを書く)
impl<'a, 'ctx> Compiler<'a, 'ctx>{

    fn new (context: &'a Context, builder: &'a Builder) -> Compiler<'a, 'ctx> where 'a: 'ctx{
        return Compiler{
            context,
            builder,
            module: None,
            types: HashMap::new(),
            stack_function: vec![],
            stack: HashMap::new()
        };
    }

    fn emit_as_text(&self) -> Option<String> {
        if let Some(module) = &self.module {
            return Some(module.print_to_string().to_string());
        }
        return None;
    }

    /// プリミティブ型を定義
    fn init_primitive_types(&mut self) {

        // Number -> f64
        self.types.insert(String::from("Number"), AnyTypeEnum::FloatType(self.context.f64_type()));

        // i32 -> i32
        self.types.insert(String::from("i32"), AnyTypeEnum::IntType(self.context.i32_type()));
        
        // bool -> i1
        self.types.insert(String::from("bool"), AnyTypeEnum::IntType(self.context.custom_width_int_type(1)));
        
        // void -> void
        self.types.insert(String::from("void"), AnyTypeEnum::VoidType(self.context.void_type()));

    }

    /// モジュールを作成
    fn create_module(&mut self, module_name: &str) {
        self.module = Some(self.context.create_module(module_name));
    }

    /// 関数を作成(宣言してブロックを作成)
    fn create_function(&mut self, name: &'a str, return_type: &str, param_types: &Vec<&str>, param_names: &Vec<&'a str>) -> FunctionValue {
        // 仮引数の型を参照
        let param_types = &param_types.iter().map(|param_type| {
            if let Some(&param_type) = self.types.get(&param_type.to_string()) {
                return match param_type {
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
        if let Some(&return_type) = self.types.get(&String::from(return_type)) {

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
                    self.stack.insert(param_name, alloca);
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
    fn create_function_declare(&mut self, name: &'a str, return_type: &str, param_types: &Vec<&str>) -> FunctionValue {

        // 仮引数の型を参照
        let param_types = &param_types.iter().map(|param_type| {
            if let Some(&param_type) = self.types.get(&param_type.to_string()) {
                return match param_type {
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
        if let Some(&return_type) = self.types.get(&String::from(return_type)) {

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
        else {
            panic!("Return type ({}) not defined!", return_type);
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
    fn create_if_branch(&self, condition_bool: IntValue) -> (BasicBlock, BasicBlock, BasicBlock) {
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
    fn end_if_branch(&self, branch: &BasicBlock) -> BasicBlock{
        self.builder.build_unconditional_branch(*branch);
        return self.builder.get_insert_block().unwrap();
    }

    /// if式を作成(マージ)
    fn merge_if_branch(&self, then_value: &BasicValueEnum, else_value: &BasicValueEnum, then_block: BasicBlock, else_block: BasicBlock, cont_block: BasicBlock, typename:&str) -> BasicValueEnum{
        self.builder.position_at_end(cont_block);
        if discriminant(then_value) != discriminant(else_value) {
            panic!("The return value on then and the return value on else have different types.");
        }
        if let Some(&rettype) = self.types.get(&String::from(typename)) {
            let rettype = match rettype {
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

    /// 変数を参照
    fn get_variable(&self, name: &str) -> BasicValueEnum {
        return self.builder.build_load(
            *self.stack.get(name)
                .unwrap_or_else(||panic!("Variable {} is not declared",name)), name);
        
    }

    /// 比較演算子
    fn create_comparison_operator(&self, op:Predicate ,left: BasicValueEnum, right: BasicValueEnum) -> IntValue {
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
    fn create_constant_number(&self,type_name: &str, number: f64) -> BasicValueEnum<'a> {
        if let Some(&constant_type) = self.types.get(&type_name.to_string()) {
            return match constant_type {
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
    fn create_binnary_operator(&self, op: BinaryOperator, left: &'a BasicValueEnum, right: &'a BasicValueEnum) -> BasicValueEnum{
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
    fn create_function_call(&self, name: &str, args: &'a Vec<BasicValueEnum>) -> Option<BasicValueEnum>{
        if self.stack_function.contains(&name) == false{
            panic!("Functionn {} not found!", name);
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


/// 意味解析関連関数 (ASTを解析して対応する関連関数にIRを書かせる)
impl<'a, 'ctx> Compiler<'a, 'ctx>{

}


fn main() {
    env::set_var("RUST_LOG", "debug");
    env_logger::init();

    let context = Context::create();
    let builder = context.create_builder();
    let mut compiler = Compiler::new(&context,&builder);

    compiler.init_primitive_types();
    compiler.create_module("main");
    compiler.create_function_declare("printNumber", "void", &vec!["Number"]);
    compiler.create_function("gcd", "Number", &vec!["Number", "Number"], &vec!["a","b"]);
    let left = compiler.get_variable("b");
    let right = compiler.create_constant_number("Number", 0.0);
    let ans = compiler.create_comparison_operator(Predicate::EQUAL, left, right);
    let (then_block, else_block, cont_block) = compiler.create_if_branch(ans);
    compiler.start_if_branch(&then_block);
    let then_val = compiler.get_variable("a");
    compiler.end_if_branch(&cont_block);
    compiler.start_if_branch(&else_block);
    let a = compiler.get_variable("a");
    let b = compiler.get_variable("b");
    let args = vec![b, compiler.create_binnary_operator(BinaryOperator::REM, &a, &b), ];
    let else_val = compiler.create_function_call("gcd", &args).unwrap();
    compiler.end_if_branch(&cont_block);
    let ret = compiler.merge_if_branch(&then_val, &else_val, then_block, else_block, cont_block, "Number");
    compiler.create_return(&Some(ret));
    compiler.create_function("main", "i32", &vec![], &vec![]);
    let a = compiler.create_constant_number("Number", 12.0);
    let b = compiler.create_constant_number("Number", 18.0);
    let args = vec![a,b];
    let ret = compiler.create_function_call("gcd", &args).unwrap();
    compiler.create_function_call("printNumber", &vec![ret]);
    compiler.create_return(&Some(compiler.create_constant_number("i32", 0.0)));

    println!("======== LLVM IR ========");
    println!("{}", compiler.emit_as_text().unwrap());
    println!("========== END ==========");
    println!("{:?}", compiler.emit_as_text().unwrap());
}


#[test]
fn basic_function_declaration_test()
{
    let context = Context::create();
    let builder = context.create_builder();
    let mut compiler = Compiler::new(&context,&builder);

    compiler.init_primitive_types();
    compiler.create_module("main");
    compiler.create_function_declare("main", "i32", &vec![]);
    compiler.create_return(&None);

    assert_eq!(compiler.emit_as_text().unwrap(), "; ModuleID = 'main'\nsource_filename = \"main\"\n\ndeclare i32 @main()\n")
}

#[test]
fn return_test()
{
    let context = Context::create();
    let builder = context.create_builder();
    let mut compiler = Compiler::new(&context,&builder);

    compiler.init_primitive_types();
    compiler.create_module("main");
    compiler.create_function("main", "i32", &vec![],&vec![]);
    compiler.create_return(&None);

    assert_eq!(compiler.emit_as_text().unwrap(), "; ModuleID = 'main'\nsource_filename = \"main\"\n\ndefine i32 @main() {\nmain:\n  ret void\n}\n")
}

#[test]
fn if_test()
{
    let context = Context::create();
    let builder = context.create_builder();
    let mut compiler = Compiler::new(&context,&builder);

    compiler.init_primitive_types();
    compiler.create_module("main");
    compiler.create_function("test", "bool", &vec!["i32", "i32"], &vec!["a","b"]);
    let left = compiler.get_variable("a");
    let right = compiler.get_variable("b");
    let ans = compiler.create_comparison_operator(Predicate::EQUAL, left, right);
    let (then_block, else_block, cont_block) = compiler.create_if_branch(ans);
    compiler.start_if_branch(&then_block);
    let then_val = compiler.create_constant_number("bool", 1.0);
    compiler.end_if_branch(&cont_block);
    compiler.start_if_branch(&else_block);
    let else_val = compiler.create_constant_number("bool", 0.0);
    compiler.end_if_branch(&cont_block);
    let ret = compiler.merge_if_branch(&then_val, &else_val, then_block, else_block, cont_block, "bool");
    compiler.create_return(&Some(ret));

    assert_eq!(compiler.emit_as_text().unwrap(), "; ModuleID = 'main'\nsource_filename = \"main\"\n\ndefine i1 @test(i32 %0, i32 %1) {\ntest:\n  %a = alloca i32\n  store i32 %0, i32* %a\n  %b = alloca i32\n  store i32 %1, i32* %b\n  %a1 = load i32, i32* %a\n  %b2 = load i32, i32* %b\n  %compared = icmp eq i32 %a1, %b2\n  %compared_val = alloca i1\n  store i1 %compared, i1* %compared_val\n  %2 = load i1, i1* %compared_val\n  %ifcond = icmp ne i1 %2, false\n  br i1 %ifcond, label %then, label %else\n\nthen:                                             ; preds = %test\n  br label %ifcont\n\nelse:                                             ; preds = %test\n  br label %ifcont\n\nifcont:                                           ; preds = %else, %then\n  %iftmp = phi i1 [ true, %then ], [ false, %else ]\n  ret i1 %iftmp\n}\n")
}