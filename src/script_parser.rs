use crate::actuator_handlers::send_message_to_actuator;
use crate::condition_parser::parse_condition;
use crate::intermittent::{start_intermittent, stop_intermittent};
use crate::helper::{broadcast_message_to_dashboard, DashboardMessageType};
use crate::script_methods::get_script;
use crate::sensor_handlers::send_message_to_sensor;
use anyhow::{anyhow, Error, Result};
use regex::Regex;
use std::collections::HashMap;
use log::{debug, warn};
use std::cell::RefCell;
use std::time::Instant;

/// Script engine resource limits
const MAX_EXECUTION_SECONDS: u64 = 300; // 5 minutes
const MAX_LOOP_ITERATIONS: u64 = 10_000;
const MAX_DELAY_MS: u64 = 60_000; // 60 seconds
const MAX_VARIABLES: usize = 100;

thread_local! {
    static SCRIPT_START_TIME: RefCell<Option<Instant>> = const { RefCell::new(None) };
}

fn is_script_timed_out() -> bool {
    SCRIPT_START_TIME.with(|t| {
        t.borrow()
            .map(|start| start.elapsed().as_secs() >= MAX_EXECUTION_SECONDS)
            .unwrap_or(false)
    })
}

type Command = &'static str;

const COMMAND_ACTIVATE_ACTUATOR: Command = "ACTIVATE";
const COMMAND_DEACTIVATE_ACTUATOR: Command = "DEACTIVATE";
const COMMAND_PULSE_ACTUATOR: Command = "PULSE";
const COMMAND_READ_SENSOR: Command = "READ";
const COMMAND_SEND_MESSAGE_TO_DASHBOARD: Command = "SEND_TO_DASHBOARD";

const COMMAND_SET_VARIABLE: Command = "SET";
const COMMAND_UNSET_VARIABLE: Command = "UNSET";

const COMMAND_ADD_TO_VARIABLE: Command = "ADD";
const COMMAND_SUBTRACT_FROM_VARIABLE: Command = "SUBTRACT";
const COMMAND_MULTIPLY_VARIABLE: Command = "MULTIPLY";
const COMMAND_DIVIDE_VARIABLE: Command = "DIVIDE";
const COMMAND_MODULO_VARIABLE: Command = "MODULO";

const COMMAND_INTERMITTENT_ACTUATOR: Command = "INTERMITTENT";
const COMMAND_STOP_INTERMITTENT_ACTUATOR: Command = "STOP_INTERMITTENT";

const COMMAND_DELAY: Command = "DELAY";

type Instruction = &'static str;

const INSTRUCTION_IF: Instruction = "IF";
const INSTRUCTION_LOOP: Instruction = "LOOP";
const INSTRUCTION_WHILE_LOOP: Instruction = "WHILE";

const INSTRUCTION_BREAK: Instruction = "BREAK";
const INSTRUCTION_CONTINUE: Instruction = "CONTINUE";

const INSTRUCTION_BLOCK_START: Instruction = "THEN";
const INSTRUCTION_BLOCK_END: Instruction = "END";

const MAIN_BLOCK_START: Instruction = "RUN";
const MAIN_BLOCK_END: Instruction = "STOP";

fn operation_to_numeric_variable_value(
    variable: &Value,
    value: f64,
    operation: Command,
) -> Result<f64> {
    if operation == COMMAND_ADD_TO_VARIABLE {
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
    } else if operation == COMMAND_SUBTRACT_FROM_VARIABLE {
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

fn change_numeric_variable_value(
    args: &Args,
    variables: &Variables,
    operation: Command,
) -> Result<(String, f64)> {
    let variable_name = match args.first().unwrap() {
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
        Value::Variable(s) => match variables.get(s) {
            Some(variable) => match variable {
                Value::Int32(s) => *s as f64,
                Value::Int64(s) => *s as f64,
                Value::Float32(s) => *s as f64,
                Value::Float64(s) => *s,
                _ => {
                    return Err(Error::msg("Invalid variable value".to_string()));
                }
            },
            None => {
                return Err(Error::msg("Variable not found".to_string()));
            }
        },
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

fn save_variable(res: CommandFunctionResult, variables: &mut Variables) -> Option<CommandFunctionResult> {
    if let CommandFunctionResult::SaveVariable(variable_name, variable_value) = res {
        if variables.len() >= MAX_VARIABLES && !variables.contains_key(&variable_name) {
            warn!("Script variable limit reached ({})", MAX_VARIABLES);
            return Some(CommandFunctionResult::Error(
                format!("Too many variables (max {})", MAX_VARIABLES),
            ));
        }
        variables.insert(variable_name, variable_value);
    }
    None
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

        final_string = final_string.replace(variable_name, variable_value.as_str());
    }

    final_string
}

fn parse_command_function(command: Command) -> CommandFunction {
    match command {
        COMMAND_ACTIVATE_ACTUATOR => Box::new(|args, _variables| {
            debug!("Activate actuator: {:?}", args);

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

            match send_message_to_actuator(actuator_id, &"ON".to_string()) {
                Ok(res) => CommandFunctionResult::Return(Value::String(res)),
                Err(e) => CommandFunctionResult::Error(e.to_string()),
            }
        }),
        COMMAND_DEACTIVATE_ACTUATOR => Box::new(|args, _variables| {
            debug!("Deactivate actuator: {:?}", args);

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

            match send_message_to_actuator(actuator_id, &"OFF".to_string()) {
                Ok(res) => CommandFunctionResult::Return(Value::String(res)),
                Err(e) => CommandFunctionResult::Error(e.to_string()),
            }
        }),
        COMMAND_PULSE_ACTUATOR => Box::new(|args, _variables| {
            debug!("Pulse actuator: {:?}", args);

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

            match send_message_to_actuator(actuator_id, &"ON-PULSE".to_string()) {
                Ok(res) => CommandFunctionResult::Return(Value::String(res)),
                Err(e) => CommandFunctionResult::Error(e.to_string()),
            }
        }),
        COMMAND_READ_SENSOR => Box::new(|args, _variables| {
            debug!("Read sensor: {:?}", args);

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

            match send_message_to_sensor(sensor_id, &"READ".to_string()) {
                Ok(res) => CommandFunctionResult::SaveVariable(
                    "$sensor_id_".to_string() + sensor_id.to_string().as_str(),
                    Value::String(res),
                ),
                Err(e) => CommandFunctionResult::Error(e.to_string()),
            }
        }),
        COMMAND_SET_VARIABLE => Box::new(|args, _variables| {
            debug!("Set variable: {:?}", args);

            match args_required(args, 2) {
                Ok(_) => {}
                Err(e) => {
                    return CommandFunctionResult::Error(e.to_string());
                }
            }
            let args = args.clone().unwrap();

            let variable_name = match args.first().unwrap() {
                Value::Variable(s) => s,
                _ => {
                    return CommandFunctionResult::Error("Invalid variable name".to_string());
                }
            };

            let variable_value = args.get(1).unwrap().clone();

            CommandFunctionResult::SaveVariable(variable_name.to_string(), variable_value)
        }),
        COMMAND_UNSET_VARIABLE => Box::new(|args, _variables| {
            debug!("Unset variable: {:?}", args);

            match args_required(args, 1) {
                Ok(_) => {}
                Err(e) => {
                    return CommandFunctionResult::Error(e.to_string());
                }
            }

            let args = args.clone().unwrap();

            let variable_name = match args.first().unwrap() {
                Value::Variable(s) => s,
                _ => {
                    return CommandFunctionResult::Error("Invalid variable name".to_string());
                }
            };

            CommandFunctionResult::SaveVariable(variable_name.to_string(), Value::None)
        }),
        COMMAND_ADD_TO_VARIABLE => Box::new(|args, variables| {
            debug!("Add to variable: {:?}", args);

            match args_required(args, 2) {
                Ok(_) => {}
                Err(e) => {
                    return CommandFunctionResult::Error(e.to_string());
                }
            }

            let args = args.clone().unwrap();

            let (variable_name, variable_value) =
                match change_numeric_variable_value(&args, variables, COMMAND_ADD_TO_VARIABLE) {
                    Ok(res) => res,
                    Err(e) => {
                        return CommandFunctionResult::Error(e.to_string());
                    }
                };

            CommandFunctionResult::SaveVariable(
                variable_name.to_string(),
                Value::Float64(variable_value),
            )
        }),
        COMMAND_SUBTRACT_FROM_VARIABLE => Box::new(|args, variables| {
            debug!("Subtract from variable: {:?}", args);

            match args_required(args, 2) {
                Ok(_) => {}
                Err(e) => {
                    return CommandFunctionResult::Error(e.to_string());
                }
            }

            let args = args.clone().unwrap();

            let (variable_name, variable_value) = match change_numeric_variable_value(
                &args,
                variables,
                COMMAND_SUBTRACT_FROM_VARIABLE,
            ) {
                Ok(res) => res,
                Err(e) => {
                    return CommandFunctionResult::Error(e.to_string());
                }
            };

            CommandFunctionResult::SaveVariable(
                variable_name.to_string(),
                Value::Float64(variable_value),
            )
        }),
        COMMAND_MULTIPLY_VARIABLE => Box::new(|args, variables| {
            debug!("Multiply variable: {:?}", args);

            match args_required(args, 2) {
                Ok(_) => {}
                Err(e) => {
                    return CommandFunctionResult::Error(e.to_string());
                }
            }

            let args = args.clone().unwrap();

            let (variable_name, variable_value) =
                match change_numeric_variable_value(&args, variables, COMMAND_MULTIPLY_VARIABLE) {
                    Ok(res) => res,
                    Err(e) => {
                        return CommandFunctionResult::Error(e.to_string());
                    }
                };

            CommandFunctionResult::SaveVariable(
                variable_name.to_string(),
                Value::Float64(variable_value),
            )
        }),
        COMMAND_DIVIDE_VARIABLE => Box::new(|args, variables| {
            debug!("Divide variable: {:?}", args);

            match args_required(args, 2) {
                Ok(_) => {}
                Err(e) => {
                    return CommandFunctionResult::Error(e.to_string());
                }
            }

            let args = args.clone().unwrap();

            let (variable_name, variable_value) =
                match change_numeric_variable_value(&args, variables, COMMAND_DIVIDE_VARIABLE) {
                    Ok(res) => res,
                    Err(e) => {
                        return CommandFunctionResult::Error(e.to_string());
                    }
                };

            CommandFunctionResult::SaveVariable(
                variable_name.to_string(),
                Value::Float64(variable_value),
            )
        }),
        COMMAND_MODULO_VARIABLE => Box::new(|args, variables| {
            debug!("Modulo variable: {:?}", args);

            match args_required(args, 2) {
                Ok(_) => {}
                Err(e) => {
                    return CommandFunctionResult::Error(e.to_string());
                }
            }

            let args = args.clone().unwrap();

            let (variable_name, variable_value) =
                match change_numeric_variable_value(&args, variables, COMMAND_MODULO_VARIABLE) {
                    Ok(res) => res,
                    Err(e) => {
                        return CommandFunctionResult::Error(e.to_string());
                    }
                };

            CommandFunctionResult::SaveVariable(
                variable_name.to_string(),
                Value::Float64(variable_value),
            )
        }),
        COMMAND_SEND_MESSAGE_TO_DASHBOARD => Box::new(|args, variables| {
            match args_required(args, 1) {
                Ok(_) => {}
                Err(e) => {
                    return CommandFunctionResult::Error(e.to_string());
                }
            }

            let args = args.clone().unwrap();

            let message = args.first().unwrap().clone();

            let message = match message {
                Value::String(s) => s,
                _ => {
                    return CommandFunctionResult::Error("Invalid message".to_string());
                }
            };

            let message = compute_message_with_variables(message, variables);

            broadcast_message_to_dashboard(message, DashboardMessageType::Info);

            CommandFunctionResult::Continue
        }),
        COMMAND_INTERMITTENT_ACTUATOR => Box::new(|args, _variables| {
            debug!("Intermittent actuator: {:?}", args);

            match args_required(args, 3) {
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

            let on_ms = match args[1] {
                Value::Int32(s) => s,
                _ => {
                    return CommandFunctionResult::Error("Invalid on_ms value".to_string());
                }
            };

            let off_ms = match args[2] {
                Value::Int32(s) => s,
                _ => {
                    return CommandFunctionResult::Error("Invalid off_ms value".to_string());
                }
            };

            start_intermittent(actuator_id, on_ms, off_ms);
            CommandFunctionResult::Continue
        }),
        COMMAND_STOP_INTERMITTENT_ACTUATOR => Box::new(|args, _variables| {
            debug!("Stop intermittent actuator: {:?}", args);

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

            stop_intermittent(actuator_id);
            CommandFunctionResult::Continue
        }),
        COMMAND_DELAY => Box::new(|args, variables| {
            match args_required(args, 1) {
                Ok(_) => {}
                Err(e) => {
                    return CommandFunctionResult::Error(e.to_string());
                }
            }

            let args = args.clone().unwrap();

            let requested = match args[0].to_string(variables).parse::<u64>() {
                Ok(v) => v,
                Err(_) => {
                    return CommandFunctionResult::Error("Invalid DELAY value".to_string());
                }
            };
            let delay = requested.min(MAX_DELAY_MS);

            std::thread::sleep(std::time::Duration::from_millis(delay));

            CommandFunctionResult::Continue
        }),
        INSTRUCTION_BREAK => Box::new(|_args, _variables| CommandFunctionResult::Break),
        INSTRUCTION_CONTINUE => Box::new(|_args, _variables| CommandFunctionResult::Continue),
        _ => Box::new(|_args, _variables| {
            CommandFunctionResult::Error("Unknown command".to_string())
        }),
    }
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

            CommandFunctionResult::Continue
        }),
        INSTRUCTION_LOOP => Box::new(|_args, inner_executions, variables| {
            let mut iterations: u64 = 0;
            loop {
                iterations += 1;
                if iterations > MAX_LOOP_ITERATIONS {
                    return CommandFunctionResult::Error(
                        format!("Loop exceeded maximum iterations ({})", MAX_LOOP_ITERATIONS),
                    );
                }
                if is_script_timed_out() {
                    return CommandFunctionResult::Error(
                        "Script execution timed out".to_string(),
                    );
                }

                let res = run_inner_executions(inner_executions, variables);

                if res.is_break() {
                    break;
                }

                if res.is_error() {
                    return res;
                }

                if res.is_save_variable() {
                    if let Some(err) = save_variable(res, variables) {
                        return err;
                    }
                    continue;
                }

                if res.is_return() {
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

            let mut iterations: u64 = 0;
            while parse_condition(args.clone().unwrap(), variables).evaluate() {
                iterations += 1;
                if iterations > MAX_LOOP_ITERATIONS {
                    return CommandFunctionResult::Error(
                        format!("While loop exceeded maximum iterations ({})", MAX_LOOP_ITERATIONS),
                    );
                }
                if is_script_timed_out() {
                    return CommandFunctionResult::Error(
                        "Script execution timed out".to_string(),
                    );
                }

                let res = run_inner_executions(inner_executions, variables);

                if res.is_break() {
                    break;
                }

                if res.is_error() {
                    return res;
                }

                if res.is_save_variable() {
                    if let Some(err) = save_variable(res, variables) {
                        return err;
                    }
                    continue;
                }

                if res.is_return() {
                    return res;
                }
            }

            CommandFunctionResult::Continue
        }),
        INSTRUCTION_BREAK => {
            Box::new(|_args, _inner_executions, _variables| CommandFunctionResult::Break)
        }
        INSTRUCTION_CONTINUE => Box::new(|_args, _inner_executions, _variables| {
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

    if number_of_args != -1i8 as isize
        && args.clone().unwrap().len() != number_of_args.unsigned_abs()
    {
        return Err(anyhow!("Invalid number of arguments"));
    }

    Ok(())
}

fn run_inner_executions(
    inner_executions: &Vec<ScriptExecution>,
    variables: &mut Variables,
) -> CommandFunctionResult {
    if is_script_timed_out() {
        return CommandFunctionResult::Error(
            format!("Script execution timed out ({}s)", MAX_EXECUTION_SECONDS),
        );
    }

    for execution in inner_executions {
        match execution {
            ScriptExecution::Command(command) => {
                let res = command.execute(variables);

                if res.is_break() {
                    return CommandFunctionResult::Break;
                }

                if res.is_error() {
                    return res;
                }

                if res.is_save_variable() {
                    if let Some(err) = save_variable(res, variables) {
                        return err;
                    }
                    continue;
                }

                if res.is_return() {
                    return res;
                }
            }
            ScriptExecution::Block(block) => {
                let res = block.execute(variables);

                if res.is_break() {
                    return CommandFunctionResult::Break;
                }

                if res.is_error() {
                    return res;
                }

                if res.is_save_variable() {
                    if let Some(err) = save_variable(res, variables) {
                        return err;
                    }
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
        Value::String(s[1..s.len() - 1].to_string())
    } else if s.starts_with("$") {
        Value::Variable(s)
    } else if (s.starts_with("[") && s.ends_with("]")) || (s.starts_with("{") && s.ends_with("}")) {
        let inner = &s[1..s.len() - 1];
        if inner.is_empty() {
            Value::Array(Vec::new())
        } else {
            Value::Array(
                inner
                    .split(",")
                    .map(|arg| parse_argument(arg.trim().to_string()))
                    .collect::<Vec<Value>>(),
            )
        }
    } else if s.contains('.') {
        if let Ok(f) = s.parse::<f64>() {
            Value::Float64(f)
        } else {
            Value::String(s)
        }
    } else if let Ok(i) = s.parse::<i32>() {
        Value::Int32(i)
    } else if let Ok(i) = s.parse::<i64>() {
        Value::Int64(i)
    } else {
        Value::String(s)
    }
}

/// Split a script line into tokens, keeping quoted spans (e.g. `"hello world"`) intact.
fn tokenize(s: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;

    for c in s.chars() {
        match c {
            '"' => {
                in_quotes = !in_quotes;
                current.push(c);
            }
            c if c.is_whitespace() && !in_quotes => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(c),
        }
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    tokens
}

fn parse_instruction(s: String) -> Result<(Instruction, Option<Args>)> {
    let mut fragments = tokenize(&s).into_iter();

    let instruction = match fragments.next() {
        Some(i) => i,
        None => return Err(anyhow!("Empty instruction")),
    };
    let arguments = fragments
        .map(|arg| parse_argument(arg))
        .collect::<Vec<Value>>();

    let i = match instruction.as_str() {
        INSTRUCTION_IF => INSTRUCTION_IF,
        INSTRUCTION_LOOP => INSTRUCTION_LOOP,
        INSTRUCTION_WHILE_LOOP => INSTRUCTION_WHILE_LOOP,
        INSTRUCTION_BREAK => INSTRUCTION_BREAK,
        INSTRUCTION_CONTINUE => INSTRUCTION_CONTINUE,
        _ => {
            return Err(anyhow!("Unknown instruction: {}", instruction));
        }
    };

    Ok((
        i,
        if !arguments.is_empty() {
            Some(arguments)
        } else {
            None
        },
    ))
}

fn parse_command(s: String) -> Result<(Command, Option<Args>)> {
    let mut fragments = tokenize(&s).into_iter();

    let command = match fragments.next() {
        Some(c) => c,
        None => return Err(anyhow!("Empty command")),
    };
    let arguments = fragments
        .map(|arg| parse_argument(arg))
        .collect::<Vec<Value>>();

    let c = match command.as_str() {
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
        COMMAND_MODULO_VARIABLE => COMMAND_MODULO_VARIABLE,
        COMMAND_INTERMITTENT_ACTUATOR => COMMAND_INTERMITTENT_ACTUATOR,
        COMMAND_STOP_INTERMITTENT_ACTUATOR => COMMAND_STOP_INTERMITTENT_ACTUATOR,
        COMMAND_DELAY => COMMAND_DELAY,
        INSTRUCTION_BREAK => INSTRUCTION_BREAK,
        INSTRUCTION_CONTINUE => INSTRUCTION_CONTINUE,
        _ => {
            return Err(anyhow!("Unknown command: {}", command));
        }
    };

    Ok((
        c,
        if !arguments.is_empty() {
            Some(arguments)
        } else {
            None
        },
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
        self.to_string_with_depth(variables, 0)
    }

    fn to_string_with_depth(&self, variables: &Variables, depth: usize) -> String {
        // Variable references may form cycles ($a -> $b -> $a). Stop resolving
        // past a bounded depth instead of recursing forever.
        const MAX_VARIABLE_DEPTH: usize = 16;

        match self {
            Value::None => "".to_string(),
            Value::String(s) => s.to_string(),
            Value::Variable(name) => {
                if depth >= MAX_VARIABLE_DEPTH {
                    return String::new();
                }

                match variables.get(name) {
                    Some(value) => value.to_string_with_depth(variables, depth + 1),
                    None => String::new(),
                }
            }
            Value::Int32(n) => n.to_string(),
            Value::Int64(n) => n.to_string(),
            Value::Float32(n) => n.to_string(),
            Value::Float64(n) => n.to_string(),
            Value::Boolean(b) => b.to_string(),
            Value::Array(array) => {
                let values = array
                    .iter()
                    .map(|value| value.to_string_with_depth(variables, depth))
                    .collect::<Vec<String>>();

                format!("[{}]", values.join(", "))
            }
        }
    }
}

pub type Args = Vec<Value>;

pub type Variables = HashMap<String, Value>;

type CommandFunction =
    Box<dyn Fn(&Option<Args>, &mut Variables) -> CommandFunctionResult + Send>;
type InstructionFunction = Box<
    dyn Fn(&Option<Args>, &Vec<ScriptExecution>, &mut Variables) -> CommandFunctionResult + Send,
>;

pub enum CommandFunctionResult {
    Error(String),
    SaveVariable(String, Value),
    Return(Value),
    Continue,
    Break,
}

impl CommandFunctionResult {
    fn is_save_variable(&self) -> bool {
        matches!(self, CommandFunctionResult::SaveVariable(_, _))
    }

    fn is_error(&self) -> bool {
        matches!(self, CommandFunctionResult::Error(_))
    }

    fn is_return(&self) -> bool {
        matches!(self, CommandFunctionResult::Return(_))
    }

    fn is_break(&self) -> bool {
        matches!(self, CommandFunctionResult::Break)
    }
}

struct ScriptCommand {
    function: CommandFunction,
    arguments: Option<Args>,
}

impl ScriptCommand {
    fn new(fragment: String) -> Result<ScriptCommand> {
        let (command, arguments) = parse_command(fragment)?;

        Ok(ScriptCommand {
            arguments: match &arguments {
                Some(args) if !args.is_empty() => Some(args.clone()),
                _ => None,
            },
            function: parse_command_function(command),
        })
    }

    fn get_arguments(&self) -> &Option<Args> {
        &self.arguments
    }

    fn get_function(&self) -> &CommandFunction {
        &self.function
    }

    fn execute(&self, variables: &mut Variables) -> CommandFunctionResult {
        self.get_function()(self.get_arguments(), variables)
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
        let (instruction, arguments) = parse_instruction(instruction_fragment)?;

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

    fn execute(&self, variables: &mut Variables) -> CommandFunctionResult {
        self.get_function()(
            self.get_arguments(),
            self.get_inner_executions(),
            variables,
        )
    }
}

enum ScriptExecution {
    Command(ScriptCommand),
    Block(ScriptBlock),
}

fn is_block_instruction(instruction: &str) -> bool {
    matches!(
        instruction,
        INSTRUCTION_IF | INSTRUCTION_LOOP | INSTRUCTION_WHILE_LOOP
    )
}

/// Parse a body of script lines. The DSL is line-oriented: every command is a
/// single line, and blocks are delimited by `THEN` and a matching `END`.
/// Nesting is tracked by depth so sequential blocks don't swallow each other.
fn parse_execution_lines(lines: &[&str]) -> Result<Vec<ScriptExecution>> {
    let mut executions = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i].trim();

        if line.is_empty() {
            i += 1;
            continue;
        }

        let tokens = tokenize(line);
        let head = tokens.first().map(String::as_str).unwrap_or_default();

        if is_block_instruction(head) {
            // The opening THEN is either the last token of this line or the
            // first token of the next non-empty line.
            let (header_tokens, inline_body, body_start) =
                if let Some(then_index) = tokens.iter().position(|t| t == INSTRUCTION_BLOCK_START) {
                    (
                        tokens[..then_index].to_vec(),
                        tokens[then_index + 1..].to_vec(),
                        i + 1,
                    )
                } else {
                    let mut j = i + 1;

                    while j < lines.len() && lines[j].trim().is_empty() {
                        j += 1;
                    }

                    if j >= lines.len() || lines[j].trim() != INSTRUCTION_BLOCK_START {
                        return Err(anyhow!("Missing block start (THEN) after {}", head));
                    }

                    (tokens.clone(), Vec::new(), j + 1)
                };

            // Find the END belonging to this block, counting nested blocks.
            let mut depth = 1;
            let mut end = None;
            let mut j = body_start;

            while j < lines.len() {
                let candidate = lines[j].trim();

                if !candidate.is_empty() {
                    let candidate_tokens = tokenize(candidate);
                    let candidate_head = candidate_tokens
                        .first()
                        .map(String::as_str)
                        .unwrap_or_default();

                    if candidate_head == INSTRUCTION_BLOCK_END {
                        depth -= 1;

                        if depth == 0 {
                            end = Some(j);
                            break;
                        }
                    } else if is_block_instruction(candidate_head) {
                        // An inline THEN opens the nested block immediately;
                        // otherwise its standalone THEN line increments depth.
                        if candidate_tokens
                            .iter()
                            .any(|t| t == INSTRUCTION_BLOCK_START)
                        {
                            depth += 1;
                        }
                    } else if candidate_head == INSTRUCTION_BLOCK_START {
                        depth += 1;
                    }
                }

                j += 1;
            }

            let end = end.ok_or_else(|| anyhow!("Missing block end (END) for {}", head))?;

            let mut inner_lines: Vec<&str> = Vec::new();
            let inline_body_line;

            if !inline_body.is_empty() {
                inline_body_line = inline_body.join(" ");
                inner_lines.push(inline_body_line.as_str());
            }

            inner_lines.extend_from_slice(&lines[body_start..end]);

            let inner_executions = parse_execution_lines(&inner_lines)?;
            let block = ScriptBlock::new(header_tokens.join(" "), inner_executions)?;

            executions.push(ScriptExecution::Block(block));
            i = end + 1;
            continue;
        }

        if head == INSTRUCTION_BLOCK_START || head == INSTRUCTION_BLOCK_END {
            return Err(anyhow!("Unexpected {}", head));
        }

        executions.push(ScriptExecution::Command(ScriptCommand::new(line.to_string())?));
        i += 1;
    }

    Ok(executions)
}

/// Parse the code of a script into executable steps. The script must start
/// with a standalone RUN line and end with a standalone STOP line; anything
/// else (including RUN/STOP inside strings) is preserved verbatim.
fn parse_script_code(script_id: i32, code: &str) -> Result<Script> {
    let lines: Vec<&str> = code.lines().collect();
    let mut start = 0;
    let mut end = lines.len();

    while start < end && lines[start].trim().is_empty() {
        start += 1;
    }

    while end > start && lines[end - 1].trim().is_empty() {
        end -= 1;
    }

    if start >= end {
        return Err(Error::msg("Script does not contain main block"));
    }

    if lines[start].trim() != MAIN_BLOCK_START {
        return Err(Error::msg("Script must start with RUN"));
    }

    if lines[end - 1].trim() != MAIN_BLOCK_END {
        return Err(Error::msg("Script must end with STOP"));
    }

    let executions = parse_execution_lines(&lines[start + 1..end - 1])?;

    Ok(Script {
        id: script_id,
        executions,
    })
}

/// Resets the per-thread script start time when the run scope exits — even on panic.
struct ScriptTimerGuard;

impl Drop for ScriptTimerGuard {
    fn drop(&mut self) {
        SCRIPT_START_TIME.with(|t| *t.borrow_mut() = None);
    }
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

        parse_script_code(script_id, script_model.get_code())
    }

    pub fn get_id(&self) -> i32 {
        self.id
    }

    pub fn run(&self) -> Result<CommandFunctionResult> {
        SCRIPT_START_TIME.with(|t| *t.borrow_mut() = Some(Instant::now()));
        let _timer_guard = ScriptTimerGuard;

        let mut variables: Variables = HashMap::new();

        let res = run_inner_executions(&self.executions, &mut variables);

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

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(code: &str) -> Result<Script> {
        parse_script_code(1, code)
    }

    fn run(code: &str) -> Result<Variables> {
        let script = parse(code)?;
        let mut variables = Variables::new();
        let result = run_inner_executions(&script.executions, &mut variables);

        match result {
            CommandFunctionResult::Error(e) => Err(Error::msg(e)),
            _ => Ok(variables),
        }
    }

    fn parse_err(code: &str) -> String {
        parse(code).err().expect("expected parse error").to_string()
    }

    fn command_arguments(execution: &ScriptExecution) -> Vec<Value> {
        match execution {
            ScriptExecution::Command(command) => {
                command.get_arguments().clone().unwrap_or_default()
            }
            ScriptExecution::Block(_) => panic!("expected a command"),
        }
    }

    // ── Tokenizer / argument parsing ────────────────────────────────────────

    #[test]
    fn tokenize_keeps_quoted_spans_together() {
        assert_eq!(
            tokenize(r#"SEND_TO_DASHBOARD "hello   world" now"#),
            vec![
                "SEND_TO_DASHBOARD".to_string(),
                "\"hello   world\"".to_string(),
                "now".to_string(),
            ]
        );
    }

    #[test]
    fn tokenize_handles_tabs_and_repeated_spaces() {
        assert_eq!(
            tokenize("SET\t$a    1"),
            vec!["SET".to_string(), "$a".to_string(), "1".to_string()]
        );
    }

    #[test]
    fn parse_argument_supports_all_value_types() {
        assert_eq!(parse_argument("42".to_string()), Value::Int32(42));
        assert_eq!(parse_argument("3.5".to_string()), Value::Float64(3.5));
        assert_eq!(parse_argument("true".to_string()), Value::Boolean(true));
        assert_eq!(parse_argument("false".to_string()), Value::Boolean(false));
        assert_eq!(
            parse_argument("$name".to_string()),
            Value::Variable("$name".to_string())
        );
        assert_eq!(
            parse_argument("\"a b\"".to_string()),
            Value::String("a b".to_string())
        );
        assert_eq!(
            parse_argument("[1, 2]".to_string()),
            Value::Array(vec![Value::Int32(1), Value::Int32(2)])
        );
        assert_eq!(
            parse_argument("not-a-number".to_string()),
            Value::String("not-a-number".to_string())
        );
    }

    // ── Main block parsing ──────────────────────────────────────────────────

    #[test]
    fn script_requires_main_block() {
        let err = parse_err("SET $a 1");
        assert!(err.contains("start with RUN"), "{err}");

        let err = parse_err("RUN\nSET $a 1");
        assert!(err.contains("end with STOP"), "{err}");

        let err = parse_err("");
        assert!(err.contains("main block"), "{err}");
    }

    #[test]
    fn script_allows_blank_lines_around_main_block() {
        let script = parse("\n\nRUN\nSET $a 1\nSTOP\n\n").unwrap();
        assert_eq!(script.get_id(), 1);
    }

    #[test]
    fn script_preserves_run_and_stop_inside_strings() {
        let script = parse("RUN\nSEND_TO_DASHBOARD \"RUN STOP\"\nSTOP").unwrap();
        let args = command_arguments(&script.executions[0]);
        assert_eq!(args[0], Value::String("RUN STOP".to_string()));
    }

    #[test]
    fn script_preserves_multiple_spaces_inside_quoted_message() {
        let script = parse("RUN\nSEND_TO_DASHBOARD \"a   b\"\nSTOP").unwrap();
        let args = command_arguments(&script.executions[0]);
        assert_eq!(args[0], Value::String("a   b".to_string()));
    }

    #[test]
    fn script_rejects_unknown_command() {
        let err = parse_err("RUN\nFLY 1\nSTOP");
        assert!(err.contains("Unknown command"), "{err}");
    }

    #[test]
    fn script_rejects_unexpected_end() {
        let err = parse_err("RUN\nEND\nSTOP");
        assert!(err.contains("Unexpected END"), "{err}");
    }

    #[test]
    fn script_rejects_missing_block_end() {
        let err = parse_err("RUN\nIF 1 == 1 THEN\nSET $a 1\nSTOP");
        assert!(err.contains("Missing block end"), "{err}");
    }

    #[test]
    fn script_rejects_missing_block_start() {
        let err = parse_err("RUN\nIF 1 == 1\nEND\nSTOP");
        assert!(err.contains("Missing block start"), "{err}");
    }

    // ── Execution: commands ─────────────────────────────────────────────────

    #[test]
    fn script_runs_multiple_sequential_commands() {
        // Regression: the old minifier collapsed newlines, so these were parsed
        // as a single SET with four arguments.
        let variables = run("RUN\nSET $a 1\nADD $a 1\nSTOP").unwrap();
        assert_eq!(variables.get("$a"), Some(&Value::Float64(2.0)));
    }

    #[test]
    fn script_runs_arithmetic_commands() {
        let variables = run(
            "RUN\n\
             SET $a 10\n\
             ADD $a 5\n\
             SUBTRACT $a 3\n\
             MULTIPLY $a 2\n\
             DIVIDE $a 4\n\
             MODULO $a 4\n\
             STOP",
        )
        .unwrap();

        // (10 + 5 - 3) * 2 / 4 = 6, 6 % 4 = 2
        assert_eq!(variables.get("$a"), Some(&Value::Float64(2.0)));
    }

    #[test]
    fn script_unset_marks_variable_as_none() {
        let variables = run("RUN\nSET $a 1\nUNSET $a\nSTOP").unwrap();
        assert_eq!(variables.get("$a"), Some(&Value::None));
    }

    #[test]
    fn script_rejects_wrong_argument_count() {
        let err = run("RUN\nSET $a\nSTOP").unwrap_err().to_string();
        assert!(err.contains("Invalid number of arguments"), "{err}");
    }

    #[test]
    fn script_rejects_undefined_variable_in_arithmetic() {
        let err = run("RUN\nADD $missing 1\nSTOP").unwrap_err().to_string();
        assert!(err.contains("Variable not found"), "{err}");
    }

    #[test]
    fn script_rejects_invalid_delay() {
        let err = run("RUN\nDELAY \"soon\"\nSTOP").unwrap_err().to_string();
        assert!(err.contains("Invalid DELAY value"), "{err}");
    }

    #[test]
    fn script_division_by_zero_does_not_panic() {
        let variables = run("RUN\nSET $a 10\nDIVIDE $a 0\nSTOP").unwrap();

        match variables.get("$a") {
            Some(Value::Float64(value)) => assert!(value.is_infinite()),
            other => panic!("expected infinite float, got {other:?}"),
        }
    }

    // ── Execution: control flow ─────────────────────────────────────────────

    #[test]
    fn script_runs_if_block() {
        let variables = run("RUN\nIF 1 == 1 THEN\nSET $a 1\nEND\nSTOP").unwrap();
        assert_eq!(variables.get("$a"), Some(&Value::Int32(1)));
    }

    #[test]
    fn script_skips_if_block_when_condition_is_false() {
        let variables = run("RUN\nIF 1 == 2 THEN\nSET $a 1\nEND\nSTOP").unwrap();
        assert!(!variables.contains_key("$a"));
    }

    #[test]
    fn script_accepts_then_on_the_following_line() {
        let variables = run("RUN\nIF 1 == 1\nTHEN\nSET $a 1\nEND\nSTOP").unwrap();
        assert_eq!(variables.get("$a"), Some(&Value::Int32(1)));
    }

    #[test]
    fn script_accepts_inline_body_after_then() {
        let variables = run("RUN\nIF 1 == 1 THEN SET $a 1\nEND\nSTOP").unwrap();
        assert_eq!(variables.get("$a"), Some(&Value::Int32(1)));
    }

    #[test]
    fn script_runs_sequential_blocks() {
        // Regression: parse_block used to consume everything up to the last
        // END, so a second top-level block made the script unparseable.
        let variables = run(
            "RUN\n\
             IF 1 == 1 THEN\n\
             SET $a 1\n\
             END\n\
             IF 1 == 1 THEN\n\
             SET $b 2\n\
             END\n\
             STOP",
        )
        .unwrap();

        assert_eq!(variables.get("$a"), Some(&Value::Int32(1)));
        assert_eq!(variables.get("$b"), Some(&Value::Int32(2)));
    }

    #[test]
    fn script_runs_nested_blocks() {
        let variables = run(
            "RUN\n\
             IF 1 == 1 THEN\n\
             IF 2 == 2 THEN\n\
             SET $nested 1\n\
             END\n\
             END\n\
             STOP",
        )
        .unwrap();

        assert_eq!(variables.get("$nested"), Some(&Value::Int32(1)));
    }

    #[test]
    fn script_skips_nested_block_when_outer_condition_is_false() {
        let variables = run(
            "RUN\n\
             IF 1 == 2 THEN\n\
             IF 2 == 2 THEN\n\
             SET $nested 1\n\
             END\n\
             END\n\
             STOP",
        )
        .unwrap();

        assert!(!variables.contains_key("$nested"));
    }

    #[test]
    fn script_while_loop_runs_until_condition_is_false() {
        let variables = run(
            "RUN\n\
             SET $i 0\n\
             WHILE $i < 3 THEN\n\
             ADD $i 1\n\
             END\n\
             STOP",
        )
        .unwrap();

        assert_eq!(variables.get("$i"), Some(&Value::Float64(3.0)));
    }

    #[test]
    fn script_loop_stops_on_break() {
        let variables = run(
            "RUN\n\
             SET $i 0\n\
             LOOP THEN\n\
             ADD $i 1\n\
             BREAK\n\
             END\n\
             STOP",
        )
        .unwrap();

        assert_eq!(variables.get("$i"), Some(&Value::Float64(1.0)));
    }

    #[test]
    fn script_loop_hits_iteration_limit() {
        let err = run("RUN\nLOOP THEN\nCONTINUE\nEND\nSTOP")
            .unwrap_err()
            .to_string();

        assert!(err.contains("exceeded maximum iterations"), "{err}");
    }

    #[test]
    fn script_parses_every_function_command() {
        let script = parse(
            "RUN\n\
             ACTIVATE 1\n\
             DEACTIVATE 1\n\
             PULSE 1\n\
             READ 2\n\
             SEND_TO_DASHBOARD \"hi\"\n\
             SET $a 1\n\
             UNSET $a\n\
             ADD $a 1\n\
             SUBTRACT $a 1\n\
             MULTIPLY $a 1\n\
             DIVIDE $a 1\n\
             MODULO $a 1\n\
             DELAY 0\n\
             INTERMITTENT 1 1 1000\n\
             STOP_INTERMITTENT 1\n\
             STOP",
        );

        assert!(script.is_ok(), "{:?}", script.err());
    }

    #[test]
    fn script_accepts_crlf_line_endings() {
        let script = parse("RUN\r\nSET $a 1\r\nADD $a 1\r\nSTOP\r\n").unwrap();
        let mut variables = Variables::new();
        let result = run_inner_executions(&script.executions, &mut variables);

        assert!(!result.is_error());
        assert_eq!(variables.get("$a"), Some(&Value::Float64(2.0)));
    }

    #[test]
    fn script_rejects_loop_without_then() {
        let err = parse_err("RUN\nLOOP\nEND\nSTOP");
        assert!(err.contains("Missing block start"), "{err}");
    }

    // ── Value rendering ─────────────────────────────────────────────────────

    #[test]
    fn value_to_string_resolves_nested_variables() {
        let mut variables = Variables::new();
        variables.insert("$a".to_string(), Value::Variable("$b".to_string()));
        variables.insert("$b".to_string(), Value::Int32(7));

        assert_eq!(Value::Variable("$a".to_string()).to_string(&variables), "7");
    }

    #[test]
    fn value_to_string_bounds_variable_cycles() {
        // Regression: the old depth check recursed into the same cycle forever.
        let mut variables = Variables::new();
        variables.insert("$a".to_string(), Value::Variable("$b".to_string()));
        variables.insert("$b".to_string(), Value::Variable("$a".to_string()));

        assert_eq!(Value::Variable("$a".to_string()).to_string(&variables), "");
    }

    #[test]
    fn value_to_string_formats_arrays_without_trailing_separator() {
        let value = Value::Array(vec![Value::Int32(1), Value::Int32(2)]);
        assert_eq!(value.to_string(&Variables::new()), "[1, 2]");
    }

    #[test]
    fn compute_message_replaces_known_variables_only() {
        let mut variables = Variables::new();
        variables.insert("$a".to_string(), Value::Int32(5));

        assert_eq!(
            compute_message_with_variables("value=$a missing=$b".to_string(), &variables),
            "value=5 missing=$b"
        );
    }
}
