use crate::{establish_connection, get_users};
use crate::Error;

pub struct Data;
type Context<'a> = poise::Context<'a, Data, Error>;

#[poise::command(slash_command, prefix_command)]
pub async fn ping(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say("Pong!").await?;
    Ok(())
}

#[poise::command(slash_command, prefix_command)]
pub async fn users_in_db(ctx: Context<'_>) -> Result<(), Error> {
    let mut output: String = "USERS IN DATABASE:\n".to_string();

    let connection = &mut establish_connection();

    for user in get_users(connection) {
        output.push_str(&format!("id: {}, points: {}\n", user.id, user.points));
    }

    ctx.say(output).await?;

    Ok(())
}

#[poise::command(prefix_command)]
pub async fn register(ctx: Context<'_>) -> Result<(), Error> {
    poise::builtins::register_application_commands_buttons(ctx).await?;
    Ok(())
}
