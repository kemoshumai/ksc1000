use inkwell::{context::Context, builder::Builder, module::Module, types::{AnyTypeEnum, BasicMetadataTypeEnum}, values::{FunctionValue, BasicValue, AnyValue, BasicValueEnum, IntValue, AnyValueEnum, PointerValue}, IntPredicate, basic_block::BasicBlock};
use std::{env, collections::HashMap, mem::discriminant};

/// コンパイラ構造体
struct Compiler<'a>{
    context: &'a Context,
    builder: &'a Builder<'a>,
    module: Option<Module<'a>>,
    types: HashMap<String, AnyTypeEnum<'a>>,
    stack_function: Vec<&'a str>,
    stack: HashMap<&'a str, PointerValue<'a>>
}

/// コンパイル関連関数 (実際にIRを書く)
impl<'a> Compiler<'a>{

    fn new(context: &'a Context, builder: &'a Builder) -> Compiler<'a>{
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
                panic!("Failed to craete function ({}). There is no Module yet. Create module first.", name);
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
        let zero_const = self.context.i8_type().const_zero();
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
    fn merge_if_branch(&self, then_value: &'a BasicValueEnum, else_value: &BasicValueEnum, then_block: BasicBlock, else_block: BasicBlock, cont_block: BasicBlock) -> BasicValueEnum{
        self.builder.position_at_end(cont_block);
        if discriminant(then_value) != discriminant(else_value) {
            panic!("The return value on then and the return value on else have different types.");
        }
        // phiはthen_valueの型と等しくなるが、そもそもelseとthenで型があってない場合エラーを吐かせているので、elseでも同じ結果になる。
        let phi = self.builder.build_phi(then_value.get_type(), "iftmp");
        phi.add_incoming(&[(then_value, then_block), (else_value, else_block)]);
        return phi.as_basic_value();
    }

}


/// 意味解析関連関数 (ASTを解析して対応する関連関数にIRを書かせる)
impl<'a> Compiler<'a>{

}


fn main() {
    env::set_var("RUST_LOG", "debug");
    env_logger::init();

    let context = Context::create();
    let builder = context.create_builder();
    let mut compiler = Compiler::new(&context,&builder);

    compiler.init_primitive_types();
    compiler.create_module("main");
    compiler.create_function("gcd", "Number", &vec!["Number", "Number"], &vec!["a","b"]);

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

    println!("{:?}", compiler.emit_as_text());

    assert_eq!(compiler.emit_as_text().unwrap(), "; ModuleID = 'main'\nsource_filename = \"main\"\n\ndefine i32 @main() {\nmain:\n  ret void\n}\n")
}