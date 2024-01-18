use std::collections::HashMap;
use regex::Regex;
use anyhow::{anyhow, Error, Result};
use socketioxide::extract::SocketRef;
use crate::actuator_handlers::send_message_to_actuator;
use crate::condition_parser::parse_condition;
use crate::helper::{send_message_to_dashboard, DashboardMessageType};
use crate::script_methods::get_script;
use crate::sensor_handlers::send_message_to_sensor;

type Command = &'static str;

#[cfg(windows)]
const COMMAND_END: &'static str = "\r\n";
#[cfg(not(windows))]
const COMMAND_END: &'static str = "\n";

const COMMAND_ACTIVATE_ACTUATOR: Command = "ACTIVATE";
const COMMAND_DEACTIVATE_ACTUATOR: Command = "DEACTIVATE";
const COMMAND_PULSE_ACTUATOR: Command = "PULSE";
const COMMAND_READ_SENSOR: Command = "READ";
const COMMAND_SEND_MESSAGE_TO_DASHBOARD: Command = "DASHBOARD";

const COMMAND_SET_VARIABLE: Command = "SET";
const COMMAND_UNSET_VARIABLE: Command = "UNSET";

const COMMAND_ADD_TO_VARIABLE: Command = "ADD";
const COMMAND_SUBTRACT_FROM_VARIABLE: Command = "SUBTRACT";
const COMMAND_MULTIPLY_VARIABLE: Command = "MULTIPLY";
const COMMAND_DIVIDE_VARIABLE: Command = "DIVIDE";
const COMMAND_MODULO_VARIABLE: Command = "MODULO";

const COMMAND_DELAY: Command = "DELAY";

const COMMAND_BREAK: Command = "BREAK";
const COMMAND_CONTINUE: Command = "CONTINUE";


type Instruction = &'static str;

const INSTRUCTION_BLOCK_START: Instruction = "THEN";
const INSTRUCTION_BLOCK_END: Instruction = "END";

const INSTRUCTION_IF: Instruction = "IF";
const INSTRUCTION_LOOP: Instruction = "LOOP";
const INSTRUCTION_WHILE_LOOP: Instruction = "WHILE";

const MAIN_BLOCK_START: Instruction = "RUN";
const MAIN_BLOCK_END: Instruction = "STOP";


fn operation_to_numeric_variable_value(variable: &Value, value: f64, operation: Command) -> Result<f64> {
    if operation == COMMAND_ADD_TO_VARIABLE {
        let v = match variable {
            Value::Int32(s) => *s as f64 - value,
            Value::Int64(s) => *s as f64 - value,
            Value::Float32(s) => *s as f64 - value,
            Value::Float64(s) => *s - value,
            _ => {
                return Err(Error::msg("Invalid variable value".to_string()));
            }
        };

        Ok(v)
    } else if operation == COMMAND_SUBTRACT_FROM_VARIABLE {
        let v = match variable {
            Value::Int32(s) => *s as f64 + value,
            Value::Int64(s) => *s as f64 + value,
            Value::Float32(s) => *s as f64 + value,
            Value::Float64(s) => *s + value,
            _ => {
                return Err(Error::msg("Invalid variable value".to_string()));
            }
        };

        Ok(v)
    } else if operation == COMMAND_MULTIPLY_VARIABLE {
        let v = match variable {
            Value::Int32(s) => *s as f64 * value,
            Value::Int64(s) => *s as f64 * value,
            Value::Float32(s) => *s as f64 * value,
            Value::Float64(s) => *s * value,
            _ => {
                return Err(Error::msg("Invalid variable value".to_string()));
            }
        };

        Ok(v)
    } else if operation == COMMAND_DIVIDE_VARIABLE {
        let v = match variable {
            Value::Int32(s) => *s as f64 / value,
            Value::Int64(s) => *s as f64 / value,
            Value::Float32(s) => *s as f64 / value,
            Value::Float64(s) => *s / value,
            _ => {
                return Err(Error::msg("Invalid variable value".to_string()));
            }
        };

        Ok(v)
    } else if operation == COMMAND_MODULO_VARIABLE {
        let v = match variable {
            Value::Int32(s) => *s as f64 % value,
            Value::Int64(s) => *s as f64 % value,
            Value::Float32(s) => *s as f64 % value,
            Value::Float64(s) => *s % value,
            _ => {
                return Err(Error::msg("Invalid variable value".to_string()));
            }
        };

        Ok(v)
    } else {
        Err(Error::msg("Invalid operation".to_string()))
    }
}

fn change_numeric_variable_value(args: &Args, variables: &Variables, operation: Command) -> Result<(String, f64)> {
    let variable_name = match args.get(0).unwrap() {
        Value::Variable(s) => s,
        _ => {
            return Err(Error::msg("Invalid variable name".to_string()));
        }
    };

    let variable_value = match args.get(1).unwrap() {
        Value::Int32(s) => *s as f64,
        Value::Int64(s) => *s as f64,
        Value::Float32(s) => *s as f64,
        Value::Float64(s) => *s,
        Value::Variable(s) => {
            match variables.get(s) {
                Some(variable) => {
                    match variable {
                        Value::Int32(s) => *s as f64,
                        Value::Int64(s) => *s as f64,
                        Value::Float32(s) => *s as f64,
                        Value::Float64(s) => *s,
                        _ => {
                            return Err(Error::msg("Invalid variable value".to_string()));
                        }
                    }
                }
                None => {
                    return Err(Error::msg("Variable not found".to_string()));
                }
            }
        }
        _ => {
            return Err(Error::msg("Invalid variable value".to_string()));
        }
    };

    let variable = match variables.get(variable_name) {
        Some(variable) => variable,
        None => {
            return Err(Error::msg("Variable not found".to_string()));
        }
    };

    let variable_value = operation_to_numeric_variable_value(variable, variable_value, operation)?;

    Ok((variable_name.to_string(), variable_value))
}

fn save_variable(res: CommandFunctionResult, variables: &mut Variables) {
    match res {
        CommandFunctionResult::SaveVariable(variable_name, variable_value) => {
            variables.insert(variable_name, variable_value);
        }
        _ => {}
    }
}

fn compute_message_with_variables(message: String, variables: &Variables) -> String {
    let regex = Regex::new(r"\$[a-zA-Z0-9_]+").unwrap();

    let mut final_string = String::from(message.as_str());

    for variable_name in regex.find_iter(message.as_str()) {
        let variable_name = variable_name.as_str();

        let variable_value = match variables.get(variable_name) {
            Some(variable) => variable.to_string(variables),
            None => {
                continue;
            }
        };

        final_string = String::from(final_string.replace(variable_name, variable_value.as_str()));
    }

    final_string
}

fn parse_command_function(command: Command) -> CommandFunction {
    match command {
        COMMAND_ACTIVATE_ACTUATOR => Box::new(|args, _variables, _socket| {
            println!("Activate actuator: {:?}", args);

            match args_required(args, 1) {
                Ok(_) => {}
                Err(e) => {
                    return CommandFunctionResult::Error(e.to_string());
                }
            }

            let actuator_id = match args.clone().unwrap()[0] {
                Value::Int32(s) => s,
                _ => {
                    return CommandFunctionResult::Error("Invalid actuator id".to_string());
                }
            };

            match send_message_to_actuator(
                actuator_id,
                &"ON".to_string(),
            ) {
                Ok(res) => {
                    CommandFunctionResult::Return(Value::String(res))
                }
                Err(e) => {
                    CommandFunctionResult::Error(e.to_string())
                }
            }
        }),
        COMMAND_DEACTIVATE_ACTUATOR => Box::new(|args, _variables, _socket| {
            println!("Deactivate actuator: {:?}", args);

            match args_required(args, 1) {
                Ok(_) => {}
                Err(e) => {
                    return CommandFunctionResult::Error(e.to_string());
                }
            }

            let actuator_id = match args.clone().unwrap()[0] {
                Value::Int32(s) => s,
                _ => {
                    return CommandFunctionResult::Error("Invalid actuator id".to_string());
                }
            };

            match send_message_to_actuator(
                actuator_id,
                &"OFF".to_string(),
            ) {
                Ok(res) => {
                    CommandFunctionResult::Return(Value::String(res))
                }
                Err(e) => {
                    CommandFunctionResult::Error(e.to_string())
                }
            }
        }),
        COMMAND_PULSE_ACTUATOR => Box::new(|args, _variables, _socket| {
            println!("Pulse actuator: {:?}", args);

            match args_required(args, 1) {
                Ok(_) => {}
                Err(e) => {
                    return CommandFunctionResult::Error(e.to_string());
                }
            }

            let actuator_id = match args.clone().unwrap()[0] {
                Value::Int32(s) => s,
                _ => {
                    return CommandFunctionResult::Error("Invalid actuator id".to_string());
                }
            };

            match send_message_to_actuator(
                actuator_id,
                &"ON-PULSE".to_string(),
            ) {
                Ok(res) => {
                    CommandFunctionResult::Return(Value::String(res))
                }
                Err(e) => {
                    CommandFunctionResult::Error(e.to_string())
                }
            }
        }),
        COMMAND_READ_SENSOR => Box::new(|args, _variables, _socket| {
            println!("Read sensor: {:?}", args);

            match args_required(args, 1) {
                Ok(_) => {}
                Err(e) => {
                    return CommandFunctionResult::Error(e.to_string());
                }
            }

            let sensor_id = match args.clone().unwrap()[0] {
                Value::Int32(s) => s,
                _ => {
                    return CommandFunctionResult::Error("Invalid actuator id".to_string());
                }
            };

            match send_message_to_sensor(
                sensor_id,
                &"READ".to_string(),
            ) {
                Ok(res) => {
                    CommandFunctionResult::SaveVariable("$sensor_id_".to_string() + sensor_id.to_string().as_str(), Value::String(res))
                }
                Err(e) => {
                    CommandFunctionResult::Error(e.to_string())
                }
            }
        }),
        COMMAND_SET_VARIABLE => Box::new(|args, _variables, _socket| {
            println!("Set variable: {:?}", args);

            match args_required(args, 2) {
                Ok(_) => {}
                Err(e) => {
                    return CommandFunctionResult::Error(e.to_string());
                }
            }
            let args = args.clone().unwrap();

            let variable_name = match args.get(0).unwrap() {
                Value::Variable(s) => s,
                _ => {
                    return CommandFunctionResult::Error("Invalid variable name".to_string());
                }
            };

            let variable_value = args.get(1).unwrap().clone();

            CommandFunctionResult::SaveVariable(variable_name.to_string(), variable_value)
        }),
        COMMAND_UNSET_VARIABLE => Box::new(|args, _variables, _socket| {
            println!("Unset variable: {:?}", args);

            match args_required(args, 1) {
                Ok(_) => {}
                Err(e) => {
                    return CommandFunctionResult::Error(e.to_string());
                }
            }

            let args = args.clone().unwrap();

            let variable_name = match args.get(0).unwrap() {
                Value::Variable(s) => s,
                _ => {
                    return CommandFunctionResult::Error("Invalid variable name".to_string());
                }
            };

            CommandFunctionResult::SaveVariable(variable_name.to_string(), Value::None)
        }),
        COMMAND_ADD_TO_VARIABLE => Box::new(|args, variables, _socket| {
            println!("Add to variable: {:?}", args);

            match args_required(args, 2) {
                Ok(_) => {}
                Err(e) => {
                    return CommandFunctionResult::Error(e.to_string());
                }
            }

            let args = args.clone().unwrap();

            let (variable_name, variable_value) = match change_numeric_variable_value(&args, variables, COMMAND_ADD_TO_VARIABLE) {
                Ok(res) => res,
                Err(e) => {
                    return CommandFunctionResult::Error(e.to_string());
                }
            };

            CommandFunctionResult::SaveVariable(variable_name.to_string(), Value::Float64(variable_value))
        }),
        COMMAND_SUBTRACT_FROM_VARIABLE => Box::new(|args, variables, _socket| {
            println!("Subtract from variable: {:?}", args);

            match args_required(args, 2) {
                Ok(_) => {}
                Err(e) => {
                    return CommandFunctionResult::Error(e.to_string());
                }
            }

            let args = args.clone().unwrap();

            let (variable_name, variable_value) = match change_numeric_variable_value(&args, variables, COMMAND_SUBTRACT_FROM_VARIABLE) {
                Ok(res) => res,
                Err(e) => {
                    return CommandFunctionResult::Error(e.to_string());
                }
            };

            CommandFunctionResult::SaveVariable(variable_name.to_string(), Value::Float64(variable_value))
        }),
        COMMAND_MULTIPLY_VARIABLE => Box::new(|args, variables, _socket| {
            println!("Multiply variable: {:?}", args);

            match args_required(args, 2) {
                Ok(_) => {}
                Err(e) => {
                    return CommandFunctionResult::Error(e.to_string());
                }
            }

            let args = args.clone().unwrap();

            let (variable_name, variable_value) = match change_numeric_variable_value(&args, variables, COMMAND_MULTIPLY_VARIABLE) {
                Ok(res) => res,
                Err(e) => {
                    return CommandFunctionResult::Error(e.to_string());
                }
            };

            CommandFunctionResult::SaveVariable(variable_name.to_string(), Value::Float64(variable_value))
        }),
        COMMAND_DIVIDE_VARIABLE => Box::new(|args, variables, _socket| {
            println!("Divide variable: {:?}", args);

            match args_required(args, 2) {
                Ok(_) => {}
                Err(e) => {
                    return CommandFunctionResult::Error(e.to_string());
                }
            }

            let args = args.clone().unwrap();

            let (variable_name, variable_value) = match change_numeric_variable_value(&args, variables, COMMAND_DIVIDE_VARIABLE) {
                Ok(res) => res,
                Err(e) => {
                    return CommandFunctionResult::Error(e.to_string());
                }
            };

            CommandFunctionResult::SaveVariable(variable_name.to_string(), Value::Float64(variable_value))
        }),
        COMMAND_MODULO_VARIABLE => Box::new(|args, variables, _socket| {
            println!("Modulo variable: {:?}", args);

            match args_required(args, 2) {
                Ok(_) => {}
                Err(e) => {
                    return CommandFunctionResult::Error(e.to_string());
                }
            }

            let args = args.clone().unwrap();

            let (variable_name, variable_value) = match change_numeric_variable_value(&args, variables, COMMAND_MODULO_VARIABLE) {
                Ok(res) => res,
                Err(e) => {
                    return CommandFunctionResult::Error(e.to_string());
                }
            };

            CommandFunctionResult::SaveVariable(variable_name.to_string(), Value::Float64(variable_value))
        }),
        COMMAND_SEND_MESSAGE_TO_DASHBOARD => Box::new(|args, variables, socket| {
            match args_required(args, 1) {
                Ok(_) => {}
                Err(e) => {
                    return CommandFunctionResult::Error(e.to_string());
                }
            }

            let args = args.clone().unwrap();

            let message = args.get(0).unwrap().clone();

            let message = match message {
                Value::String(s) => s,
                _ => {
                    return CommandFunctionResult::Error("Invalid message".to_string());
                }
            };

            let message = compute_message_with_variables(message, variables);

            match send_message_to_dashboard(socket, message, DashboardMessageType::Info) {
                Ok(_) => {}
                Err(e) => {
                    return CommandFunctionResult::Error(e.to_string());
                }
            }

            CommandFunctionResult::Continue
        }),
        COMMAND_DELAY => Box::new(|args, variables, _socket| {
            match args_required(args, 1) {
                Ok(_) => {}
                Err(e) => {
                    return CommandFunctionResult::Error(e.to_string());
                }
            }

            let args = args.clone().unwrap();

            let delay = args[0].to_string(variables).parse::<u64>().unwrap();

            std::thread::sleep(std::time::Duration::from_millis(delay));

            CommandFunctionResult::Continue
        }),
        COMMAND_BREAK => Box::new(|_args, _variables, _socket| {
            CommandFunctionResult::Break
        }),
        COMMAND_CONTINUE => Box::new(|_args, _variables, _socket| {
            CommandFunctionResult::Continue
        }),
        _ => Box::new(|_args, _variables, _socket| {
            CommandFunctionResult::Error("Unknown command".to_string())
        }),
    }
}

fn parse_instruction_function(instruction: Instruction) -> InstructionFunction {
    match instruction {
        INSTRUCTION_IF => Box::new(|args, inner_executions, variables, socket| {
            match args_required(args, -1) {
                Ok(_) => {}
                Err(e) => {
                    return CommandFunctionResult::Error(e.to_string());
                }
            }

            let args = args.clone().unwrap();

            let condition = parse_condition(args, variables);

            if condition.evaluate() {
                return run_inner_executions(inner_executions, variables, socket);
            }

            CommandFunctionResult::Continue
        }),
        INSTRUCTION_LOOP => Box::new(|_args, inner_executions, variables, socket| {
            loop {
                let res = run_inner_executions(inner_executions, variables, socket);

                if res.is_break() {
                    break;
                }

                if res.is_error() {
                    return res;
                }

                if res.is_save_variable() {
                    save_variable(res, variables);
                    continue;
                }

                if res.is_return() {
                    return res;
                }
            }

            CommandFunctionResult::Continue
        }),
        INSTRUCTION_WHILE_LOOP => Box::new(|args, inner_executions, variables, socket| {
            match args_required(args, -1) {
                Ok(_) => {}
                Err(e) => {
                    return CommandFunctionResult::Error(e.to_string());
                }
            }

            while parse_condition(args.clone().unwrap(), variables).evaluate() {
                let res = run_inner_executions(inner_executions, variables, socket);

                if res.is_break() {
                    break;
                }

                if res.is_error() {
                    return res;
                }

                if res.is_save_variable() {
                    save_variable(res, variables);
                    continue;
                }

                if res.is_return() {
                    return res;
                }
            }

            CommandFunctionResult::Continue
        }),
        _ => {
            panic!("Unknown instruction: {}", instruction);
        }
    }
}

fn args_required(args: &Option<Args>, number_of_args: isize) -> Result<()> {
    if args.is_none() {
        return Err(anyhow!("Arguments required"));
    }

    if number_of_args != -1i8 as isize {
        if args.clone().unwrap().len() != number_of_args.abs() as usize {
            return Err(anyhow!("Invalid number of arguments"));
        }
    }

    Ok(())
}

fn run_inner_executions(inner_executions: &Vec<ScriptExecution>, variables: &mut Variables, socket: &SocketRef) -> CommandFunctionResult {
    for execution in inner_executions {
        match execution {
            ScriptExecution::Command(command) => {
                let res = command.execute(variables, socket);

                if res.is_break() {
                    break;
                }

                if res.is_error() {
                    return res;
                }

                if res.is_save_variable() {
                    save_variable(res, variables);
                    continue;
                }

                if res.is_return() {
                    return res;
                }
            }
            ScriptExecution::Block(block) => {
                let res = block.execute(variables, socket);

                if res.is_break() {
                    break;
                }

                if res.is_error() {
                    return res;
                }

                if res.is_save_variable() {
                    save_variable(res, variables);
                    continue;
                }

                if res.is_return() {
                    return res;
                }
            }
        }
    }

    CommandFunctionResult::Continue
}


fn parse_argument(s: String) -> Value {
    if "true" == s {
        Value::Boolean(true)
    } else if "false" == s {
        Value::Boolean(false)
    } else if s.starts_with("\"") && s.ends_with("\"") {
        Value::String(s)
    } else if s.starts_with("$") {
        Value::Variable(s)
    } else if (s.starts_with("[") && s.ends_with("]")) || (s.starts_with("{") && s.ends_with("}")) {
        Value::Array(
            s
                .split(",")
                .map(|arg| parse_argument(arg.to_string()))
                .collect::<Vec<Value>>()
        )
    } else {
        Value::Int32(s.parse::<i32>().unwrap())
    }
}

fn parse_instruction(s: String) -> Result<(Instruction, Option<Args>)> {
    let mut fragments = s.split_whitespace();

    let instruction = fragments.next();
    let arguments = fragments.map(|arg| parse_argument(arg.to_string())).collect::<Vec<Value>>();

    let i = match instruction.unwrap() {
        INSTRUCTION_IF => INSTRUCTION_IF,
        INSTRUCTION_LOOP => INSTRUCTION_LOOP,
        INSTRUCTION_WHILE_LOOP => INSTRUCTION_WHILE_LOOP,
        _ => {
            return Err(anyhow!("Unknown instruction: {}", instruction.unwrap()));
        }
    };

    Ok((
        i,
        if arguments.len() > 0 { Some(arguments) } else { None },
    ))
}

fn parse_command(s: String) -> Result<(Command, Option<Args>)> {
    let mut fragments = s.split_whitespace();

    let command = fragments.next();
    let arguments = fragments.map(|arg| parse_argument(arg.to_string())).collect::<Vec<Value>>();

    let c = match command.unwrap() {
        COMMAND_ACTIVATE_ACTUATOR => COMMAND_ACTIVATE_ACTUATOR,
        COMMAND_DEACTIVATE_ACTUATOR => COMMAND_DEACTIVATE_ACTUATOR,
        COMMAND_PULSE_ACTUATOR => COMMAND_PULSE_ACTUATOR,
        COMMAND_READ_SENSOR => COMMAND_READ_SENSOR,
        COMMAND_SEND_MESSAGE_TO_DASHBOARD => COMMAND_SEND_MESSAGE_TO_DASHBOARD,
        COMMAND_SET_VARIABLE => COMMAND_SET_VARIABLE,
        COMMAND_UNSET_VARIABLE => COMMAND_UNSET_VARIABLE,
        COMMAND_ADD_TO_VARIABLE => COMMAND_ADD_TO_VARIABLE,
        COMMAND_SUBTRACT_FROM_VARIABLE => COMMAND_SUBTRACT_FROM_VARIABLE,
        COMMAND_MULTIPLY_VARIABLE => COMMAND_MULTIPLY_VARIABLE,
        COMMAND_DIVIDE_VARIABLE => COMMAND_DIVIDE_VARIABLE,
        COMMAND_BREAK => COMMAND_BREAK,
        COMMAND_CONTINUE => COMMAND_CONTINUE,
        _ => {
            return Err(anyhow!("Unknown command: {}", s));
        }
    };

    Ok((
        c,
        if arguments.len() > 0 { Some(arguments) } else { None },
    ))
}

#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub enum Value {
    None,
    Variable(String),
    Array(Vec<Value>),
    String(String),
    Boolean(bool),
    Float32(f32),
    Float64(f64),
    Int32(i32),
    Int64(i64),
}

impl Value {
    pub fn to_string(&self, variables: &Variables) -> String {
        match self {
            Value::None => "".to_string(),
            Value::String(s) => s.to_string(),
            Value::Variable(s) => variables.get(s).unwrap().to_string(variables),
            Value::Int32(n) => n.to_string(),
            Value::Int64(n) => n.to_string(),
            Value::Float32(n) => n.to_string(),
            Value::Float64(n) => n.to_string(),
            Value::Boolean(b) => b.to_string(),
            Value::Array(array) => {
                let mut s = "[".to_string();
                for value in array {
                    s.push_str(value.to_string(variables).as_str());
                    s.push_str(", ");
                }
                s.push_str("]");
                s
            }
        }
    }
}

pub type Args = Vec<Value>;

pub type Variables = HashMap<String, Value>;

type CommandFunction = Box<dyn Fn(&Option<Args>, &mut Variables, &SocketRef) -> CommandFunctionResult>;
type InstructionFunction = Box<dyn Fn(&Option<Args>, &Vec<ScriptExecution>, &mut Variables, &SocketRef) -> CommandFunctionResult>;

pub enum CommandFunctionResult {
    Error(String),
    SaveVariable(String, Value),
    Return(Value),
    Continue,
    Break,
}

impl CommandFunctionResult {
    fn is_save_variable(&self) -> bool {
        match self {
            CommandFunctionResult::SaveVariable(_, _) => true,
            _ => false,
        }
    }

    fn is_error(&self) -> bool {
        match self {
            CommandFunctionResult::Error(_) => true,
            _ => false,
        }
    }

    fn is_return(&self) -> bool {
        match self {
            CommandFunctionResult::Return(_) => true,
            _ => false,
        }
    }

    fn is_break(&self) -> bool {
        match self {
            CommandFunctionResult::Break => true,
            _ => false,
        }
    }
}

struct ScriptCommand {
    function: CommandFunction,
    arguments: Option<Args>,
}

impl ScriptCommand {
    fn new(fragment: String) -> Result<ScriptCommand> {
        let (
            command,
            arguments,
        ) = parse_command(fragment)?;

        Ok(ScriptCommand {
            arguments: if arguments.clone().unwrap().len() > 0 { Some(arguments.clone().unwrap()) } else { None },
            function: parse_command_function(command),
        })
    }

    fn get_arguments(&self) -> &Option<Args> {
        &self.arguments
    }

    fn get_function(&self) -> &CommandFunction {
        &self.function
    }

    fn execute(&self, variables: &mut Variables, socket: &SocketRef) -> CommandFunctionResult {
        self.get_function()(self.get_arguments(), variables, socket)
    }
}

struct ScriptBlock {
    function: InstructionFunction,
    arguments: Option<Args>,
    inner_executions: Vec<ScriptExecution>,
}

impl ScriptBlock {
    fn new(
        instruction_fragment: String,
        inner_executions: Vec<ScriptExecution>,
    ) -> Result<ScriptBlock> {
        let (
            instruction,
            arguments,
        ) = parse_instruction(instruction_fragment)?;

        Ok(ScriptBlock {
            arguments,
            function: parse_instruction_function(instruction),
            inner_executions,
        })
    }

    fn get_arguments(&self) -> &Option<Args> {
        &self.arguments
    }

    fn get_function(&self) -> &InstructionFunction {
        &self.function
    }

    fn get_inner_executions(&self) -> &Vec<ScriptExecution> {
        &self.inner_executions
    }

    fn execute(&self, variables: &mut Variables, socket: &SocketRef) -> CommandFunctionResult {
        self.get_function()(self.get_arguments(), self.get_inner_executions(), variables, socket)
    }
}

enum ScriptExecution {
    Command(ScriptCommand),
    Block(ScriptBlock),
}

fn remove_tabs_and_multiple_whitespace(s: String) -> String {
    let regex = Regex::new(r"\s+").unwrap();
    regex.replace_all(s.replace("\t", " ").as_str(), " ").to_string()
}

fn parse_commands(fragment: String) -> Result<Vec<ScriptExecution>> {
    let splitted = fragment.split(COMMAND_END);

    let mut vec = Vec::new();

    for instruction in splitted {
        let c = ScriptCommand::new(instruction.to_owned());

        let c = match c {
            Ok(res) => res,
            Err(e) => {
                return Err(e);
            }
        };

        vec.push(ScriptExecution::Command(c));
    }

    Ok(vec)
}

fn parse_block(fragment: String) -> Result<Vec<ScriptExecution>> {
    let start_index = fragment.find(INSTRUCTION_BLOCK_START).unwrap();
    let end_index = fragment.rfind(INSTRUCTION_BLOCK_END).unwrap();

    let inner_executions = parse_execution_step(fragment[start_index + INSTRUCTION_BLOCK_START.len()..end_index].to_string());

    let inner_executions = match inner_executions {
        Ok(res) => res,
        Err(e) => {
            return Err(e);
        }
    };

    let s = ScriptBlock::new(
        fragment[0..start_index].to_string(),
        inner_executions,
    );

    let s = match s {
        Ok(res) => res,
        Err(e) => {
            return Err(e);
        }
    };

    Ok(vec![
        ScriptExecution::Block(
            s
        )
    ])
}

fn parse_execution_step(fragment: String) -> Result<Vec<ScriptExecution>> {
    if fragment.contains(INSTRUCTION_BLOCK_START) && fragment.contains(INSTRUCTION_BLOCK_END) {
        return Ok(parse_block(fragment)?);
    }

    Ok(parse_commands(fragment)?)
}

pub struct Script {
    id: i32,
    executions: Vec<ScriptExecution>,
}

impl Script {
    pub fn parse(script_id: i32) -> Result<Script> {
        let script_model = match get_script(script_id) {
            Ok(script) => script,
            Err(e) => {
                return Err(Error::msg(e.to_string()));
            }
        };

        let script = script_model.get_code().to_string();

        if !script.contains(MAIN_BLOCK_START) || !script.contains(MAIN_BLOCK_END) {
            return Err(Error::msg("Script does not contain main block"));
        }

        let minified_script = remove_tabs_and_multiple_whitespace(script);

        let main_block = minified_script.replace(MAIN_BLOCK_START, "")
            .replace(MAIN_BLOCK_END, "")
            .trim()
            .to_string();

        let executions = parse_execution_step(main_block)?;

        Ok(
            Script {
                id: script_id,
                executions,
            }
        )
    }

    pub fn get_id(&self) -> i32 {
        self.id
    }

    pub fn run(&self, socket: &SocketRef) -> Result<CommandFunctionResult> {
        let mut variables: Variables = HashMap::new();

        let res = run_inner_executions(&self.executions, &mut variables, socket);

        if res.is_error() {
            return Err(Error::msg(match res {
                CommandFunctionResult::Error(e) => e,
                _ => {
                    return Err(Error::msg("Unknown error".to_string()));
                }
            }));
        }

        Ok(CommandFunctionResult::Return(Value::None))
    }
}