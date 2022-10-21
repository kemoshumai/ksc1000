use inkwell::{context::Context, builder::Builder, module::Module, types::{AnyTypeEnum, BasicMetadataTypeEnum}, values::FunctionValue};
use std::{env, collections::HashMap};

/// コンパイラ構造体
struct Compiler<'ctx>{
    context: &'ctx Context,
    builder: Builder<'ctx>,
    module: Option<Module<'ctx>>,
    types: HashMap<String, AnyTypeEnum<'ctx>>
}

/// コンパイル関連関数 (実際にIRを書く)
impl<'ctx> Compiler<'ctx>{

    fn new(context: &'ctx Context) -> Compiler<'ctx>{
        return Compiler{
            context,
            builder: context.create_builder(),
            module: None,
            types: HashMap::new()
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
    fn create_function(&self, name: &str, return_type: &str, param_types: Vec<&str>) -> FunctionValue {
        let func = self.create_function_declare(name, return_type, param_types);
        let basic_block = self.context.append_basic_block(func, name);
        self.builder.position_at_end(basic_block);
        return func;
    }

    /// 関数を作成(宣言のみ)
    fn create_function_declare(&self, name: &str, return_type: &str, param_types: Vec<&str>) -> FunctionValue {

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

}


/// 意味解析関連関数 (ASTを解析して対応する関連関数にIRを書かせる)
impl<'ctx> Compiler<'ctx>{

}


fn main() {
    env::set_var("RUST_LOG", "debug");
    env_logger::init();

    let mut context = Context::create();
    let mut compiler = Compiler::new(&mut context);

    compiler.init_primitive_types();
    compiler.create_module("main");
    compiler.create_function("gcd", "Number", vec!["Number", "Number"]);

    println!("======== LLVM IR ========");
    println!("{}", compiler.emit_as_text().unwrap());
    println!("========== END ==========");
}


#[test]
fn basic_function_declaration_test()
{
    let mut context = Context::create();
    let mut compiler = Compiler::new(&mut context);

    compiler.init_primitive_types();
    compiler.create_module("main");
    compiler.create_function_declare("main", "i32", vec![]);

    assert_eq!(compiler.emit_as_text().unwrap(), "; ModuleID = 'main'\nsource_filename = \"main\"\n\ndeclare i32 @main()\n")
}