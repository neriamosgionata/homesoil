use std::collections::HashMap;
use regex::Regex;
use anyhow::{anyhow, Error, Result};
use crate::actuator_handlers::send_message_to_actuator;
use crate::condition_parser::parse_condition;
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
const COMMAND_SET_VARIABLE: Command = "SET";
const COMMAND_UNSET_VARIABLE: Command = "UNSET";

const COMMAND_BREAK: Command = "BREAK";
const COMMAND_CONTINUE: Command = "CONTINUE";


type Instruction = &'static str;

const INSTRUCTION_BLOCK_START: Instruction = "THEN";
const INSTRUCTION_BLOCK_END: Instruction = "END";

const INSTRUCTION_IF: Instruction = "IF";
const INSTRUCTION_LOOP: Instruction = "LOOP";
const INSTRUCTION_WHILE_LOOP: Instruction = "WHILE";
const INSTRUCTION_DELAY: Instruction = "DELAY";

const MAIN_BLOCK_START: Instruction = "RUN";
const MAIN_BLOCK_END: Instruction = "STOP";

fn parse_command_function(command: Command) -> CommandFunction {
    match command {
        COMMAND_ACTIVATE_ACTUATOR => Box::new(|args, variables| {
            println!("Activate actuator: {:?}", args);

            match args_required(args, 1) {
                Ok(_) => {}
                Err(e) => {
                    return CommandFunctionResult::Error(e.to_string());
                }
            }

            let args = args.clone().unwrap();

            let actuator_id = match args[0] {
                Value::Int32(s) => s,
                _ => {
                    return CommandFunctionResult::Error("Invalid actuator id".to_string());
                }
            };

            let res = send_message_to_actuator(
                actuator_id,
                &"ON".to_string(),
            );

            match res {
                Ok(res) => {
                    CommandFunctionResult::Return(Value::String(res))
                }
                Err(e) => {
                    CommandFunctionResult::Error(e.to_string())
                }
            }
        }),
        COMMAND_DEACTIVATE_ACTUATOR => Box::new(|args, variables| {
            println!("Deactivate actuator: {:?}", args);

            match args_required(args, 1) {
                Ok(_) => {}
                Err(e) => {
                    return CommandFunctionResult::Error(e.to_string());
                }
            }

            let args = args.clone().unwrap();

            let actuator_id = match args[0] {
                Value::Int32(s) => s,
                _ => {
                    return CommandFunctionResult::Error("Invalid actuator id".to_string());
                }
            };

            let res = send_message_to_actuator(
                actuator_id,
                &"OFF".to_string(),
            );

            match res {
                Ok(res) => {
                    CommandFunctionResult::Return(Value::String(res))
                }
                Err(e) => {
                    CommandFunctionResult::Error(e.to_string())
                }
            }
        }),
        COMMAND_PULSE_ACTUATOR => Box::new(|args, variables| {
            println!("Pulse actuator: {:?}", args);

            match args_required(args, 1) {
                Ok(_) => {}
                Err(e) => {
                    return CommandFunctionResult::Error(e.to_string());
                }
            }

            let args = args.clone().unwrap();

            let actuator_id = match args[0] {
                Value::Int32(s) => s,
                _ => {
                    return CommandFunctionResult::Error("Invalid actuator id".to_string());
                }
            };

            let res = send_message_to_actuator(
                actuator_id,
                &"ON-PULSE".to_string(),
            );

            match res {
                Ok(res) => {
                    CommandFunctionResult::Return(Value::String(res))
                }
                Err(e) => {
                    CommandFunctionResult::Error(e.to_string())
                }
            }
        }),
        COMMAND_READ_SENSOR => Box::new(|args, variables| {
            println!("Read sensor: {:?}", args);

            match args_required(args, 1) {
                Ok(_) => {}
                Err(e) => {
                    return CommandFunctionResult::Error(e.to_string());
                }
            }

            let args = args.clone().unwrap();

            let sensor_id = match args[0] {
                Value::Int32(s) => s,
                _ => {
                    return CommandFunctionResult::Error("Invalid actuator id".to_string());
                }
            };

            let res = send_message_to_sensor(
                sensor_id,
                &"READ".to_string(),
            );

            match res {
                Ok(res) => {
                    CommandFunctionResult::Return(Value::String(res))
                }
                Err(e) => {
                    CommandFunctionResult::Error(e.to_string())
                }
            }
        }),
        COMMAND_SET_VARIABLE => Box::new(|args, variables| {
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

            let variable_value = args[1].clone();

            CommandFunctionResult::SaveVariable(variable_name.to_string(), variable_value)
        }),
        COMMAND_UNSET_VARIABLE => Box::new(|args, variables| {
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
        COMMAND_BREAK => Box::new(|_args, _variables| {
            CommandFunctionResult::Break
        }),
        COMMAND_CONTINUE => Box::new(|_args, _variables| {
            CommandFunctionResult::Continue
        }),
        _ => Box::new(|_args, _variables| {
            CommandFunctionResult::Error("Unknown command".to_string())
        }),
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

fn run_inner_executions(inner_executions: &Vec<ScriptExecution>, variables: &Variables) -> CommandFunctionResult {
    for execution in inner_executions {
        match execution {
            ScriptExecution::Command(command) => {
                let execution = command.execute(variables);
                if execution.is_break() {
                    break;
                }

                if execution.is_error() {
                    return execution;
                }
            }
            ScriptExecution::Block(block) => {
                let execution = block.execute(variables);
                if execution.is_break() {
                    break;
                }

                if execution.is_error() {
                    return execution;
                }
            }
        }
    }

    CommandFunctionResult::Continue
}

fn parse_instruction_function(instruction: Instruction) -> InstructionFunction {
    match instruction {
        INSTRUCTION_IF => Box::new(|args, inner_executions, variables| {
            match args_required(args, -1) {
                Ok(_) => {}
                Err(e) => {
                    return CommandFunctionResult::Error(e.to_string());
                }
            }

            let args = args.clone().unwrap();

            let condition = parse_condition(args, variables);

            if condition.evaluate() {
                return run_inner_executions(inner_executions, variables);
            }

            CommandFunctionResult::Error("Condition not met".to_string())
        }),
        INSTRUCTION_LOOP => Box::new(|_args, inner_executions, variables| {
            loop {
                let res = run_inner_executions(inner_executions, variables);

                if res.is_break() {
                    break;
                }

                if res.is_error() {
                    return res;
                }
            }

            CommandFunctionResult::Continue
        }),
        INSTRUCTION_WHILE_LOOP => Box::new(|args, inner_executions, variables| {
            match args_required(args, -1) {
                Ok(_) => {}
                Err(e) => {
                    return CommandFunctionResult::Error(e.to_string());
                }
            }

            while parse_condition(args.clone().unwrap(), variables).evaluate() {
                let res = run_inner_executions(inner_executions, variables);

                if res.is_break() {
                    break;
                }

                if res.is_error() {
                    return res;
                }
            }

            CommandFunctionResult::Continue
        }),
        INSTRUCTION_DELAY => Box::new(|args, _inner_executions, variables| {
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
        _ => {
            panic!("Unknown instruction: {}", instruction);
        }
    }
}

fn parse_argument(s: String) -> Value {
    match s.as_str() {
        "true" => Value::Boolean(true),
        "false" => Value::Boolean(false),
        _ => {
            if s.starts_with("\"") && s.ends_with("\"") {
                Value::String(s[1..s.len() - 1].to_string())
            } else if s.starts_with("$") {
                Value::Variable(s[1..s.len()].to_string())
            } else if (s.starts_with("[") && s.ends_with("]")) || (s.starts_with("{") && s.ends_with("}")) {
                Value::Array(
                    s[1..s.len() - 1]
                        .split(",")
                        .map(|arg| parse_argument(arg.to_string()))
                        .collect::<Vec<Value>>()
                )
            } else {
                Value::Int32(s.parse::<i32>().unwrap())
            }
        }
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
        INSTRUCTION_DELAY => INSTRUCTION_DELAY,
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
        COMMAND_SET_VARIABLE => COMMAND_SET_VARIABLE,
        COMMAND_UNSET_VARIABLE => COMMAND_UNSET_VARIABLE,
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
    Array(Vec<Value>),
    None,
    String(String),
    Variable(String),
    Float32(f32),
    Float64(f64),
    Int32(i32),
    Int64(i64),
    Int128(i128),
    Boolean(bool),
}

impl Value {
    pub fn to_string(&self, variables: &Variables) -> String {
        match self {
            Value::None => "".to_string(),
            Value::String(s) => s.to_string(),
            Value::Variable(s) => variables.get(s).expect(format!("Variable {} not found", s).as_str()).to_string(variables),
            Value::Int32(n) => n.to_string(),
            Value::Int64(n) => n.to_string(),
            Value::Int128(n) => n.to_string(),
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

type CommandFunction = Box<dyn Fn(&Option<Args>, &Variables) -> CommandFunctionResult>;
type InstructionFunction = Box<dyn Fn(&Option<Args>, &Vec<ScriptExecution>, &Variables) -> CommandFunctionResult>;

enum CommandFunctionResult {
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

    fn is_continue(&self) -> bool {
        match self {
            CommandFunctionResult::Continue => true,
            _ => false,
        }
    }
}

struct ScriptCommand {
    command: Command,
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
            command,
            arguments: if arguments.clone().unwrap().len() > 0 { Some(arguments.clone().unwrap()) } else { None },
            function: parse_command_function(command),
        })
    }

    fn get_command(&self) -> Instruction {
        self.command
    }

    fn get_arguments(&self) -> &Option<Args> {
        &self.arguments
    }

    fn get_function(&self) -> &CommandFunction {
        &self.function
    }

    fn execute(&self, variables: &Variables) -> CommandFunctionResult {
        self.get_function()(self.get_arguments(), variables)
    }
}

struct ScriptBlock {
    instruction: Instruction,
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
            instruction,
            arguments,
            function: parse_instruction_function(instruction),
            inner_executions,
        })
    }

    fn get_instruction(&self) -> Instruction {
        self.instruction
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

    fn execute(&self, variables: &Variables) -> CommandFunctionResult {
        self.get_function()(self.get_arguments(), self.get_inner_executions(), variables)
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
    executions: Vec<ScriptExecution>,
}

impl Script {
    pub fn new(script: String) -> Result<Script> {
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
                executions,
            }
        )
    }

    pub fn run(&self) {
        let variables: Variables = HashMap::new();

        for execution in &self.executions {
            match execution {
                ScriptExecution::Command(command) => {
                    command.execute(&variables);
                }
                ScriptExecution::Block(block) => {
                    block.execute(&variables);
                }
            }
        }
    }
}