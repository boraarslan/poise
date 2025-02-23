//! Holds application command definition structs.

use crate::{serenity_prelude as serenity, BoxFuture, Framework};

/// Abstracts over a refernce to an application command interaction or autocomplete interaction
///
/// Used in [`crate::ApplicationContext`]. We need to support autocomplete interactions in
/// [`crate::ApplicationContext`] because command checks are invoked for autocomplete interactions
/// too: we don't want poise accidentally leaking sensitive information through autocomplete
/// suggestions
#[derive(Copy, Clone, Debug)]
pub enum ApplicationCommandOrAutocompleteInteraction<'a> {
    /// An application command interaction
    ApplicationCommand(&'a serenity::ApplicationCommandInteraction),
    /// An autocomplete interaction
    Autocomplete(&'a serenity::AutocompleteInteraction),
}

impl<'a> ApplicationCommandOrAutocompleteInteraction<'a> {
    /// Returns the data field of the underlying interaction
    pub fn data(self) -> &'a serenity::ApplicationCommandInteractionData {
        match self {
            Self::ApplicationCommand(x) => &x.data,
            Self::Autocomplete(x) => &x.data,
        }
    }

    /// Returns the ID of the underlying interaction
    pub fn id(self) -> serenity::InteractionId {
        match self {
            Self::ApplicationCommand(x) => x.id,
            Self::Autocomplete(x) => x.id,
        }
    }

    /// Returns the guild ID of the underlying interaction
    pub fn guild_id(self) -> Option<serenity::GuildId> {
        match self {
            Self::ApplicationCommand(x) => x.guild_id,
            Self::Autocomplete(x) => x.guild_id,
        }
    }

    /// Returns the channel ID of the underlying interaction
    pub fn channel_id(self) -> serenity::ChannelId {
        match self {
            Self::ApplicationCommand(x) => x.channel_id,
            Self::Autocomplete(x) => x.channel_id,
        }
    }

    /// Returns the member field of the underlying interaction
    pub fn member(self) -> Option<&'a serenity::Member> {
        match self {
            Self::ApplicationCommand(x) => x.member.as_ref(),
            Self::Autocomplete(x) => x.member.as_ref(),
        }
    }

    /// Returns the user field of the underlying interaction
    pub fn user(self) -> &'a serenity::User {
        match self {
            Self::ApplicationCommand(x) => &x.user,
            Self::Autocomplete(x) => &x.user,
        }
    }

    /// Returns the inner [`serenity::ApplicationCommandInteraction`] and panics otherwise
    pub fn unwrap(self) -> &'a serenity::ApplicationCommandInteraction {
        match self {
            ApplicationCommandOrAutocompleteInteraction::ApplicationCommand(x) => x,
            ApplicationCommandOrAutocompleteInteraction::Autocomplete(_) => {
                panic!("expected application command interaction, got autocomplete interaction")
            }
        }
    }
}

/// Application command specific context passed to command invocations.
pub struct ApplicationContext<'a, U, E> {
    /// Serenity's context, like HTTP or cache
    pub discord: &'a serenity::Context,
    /// The interaction which triggered this command execution.
    pub interaction: ApplicationCommandOrAutocompleteInteraction<'a>,
    /// Slash command arguments
    ///
    /// **Not** equivalent to `self.interaction.data().options`. That one refers to just the
    /// top-level command arguments, whereas [`Self::args`] is the options of the actual
    /// subcommand, if any.
    pub args: &'a [serenity::ApplicationCommandInteractionDataOption],
    /// Keeps track of whether an initial response has been sent.
    ///
    /// Discord requires different HTTP endpoints for initial and additional responses.
    pub has_sent_initial_response: &'a std::sync::atomic::AtomicBool,
    /// Read-only reference to the framework
    ///
    /// Useful if you need the list of commands, for example for a custom help command
    pub framework: &'a Framework<U, E>,
    /// The command object which is the current command
    pub command: &'a crate::Command<U, E>,
    /// Your custom user data
    pub data: &'a U,
}
impl<U, E> Clone for ApplicationContext<'_, U, E> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<U, E> Copy for ApplicationContext<'_, U, E> {}
impl<U, E> crate::_GetGenerics for ApplicationContext<'_, U, E> {
    type U = U;
    type E = E;
}

impl<U: std::fmt::Debug, E: std::fmt::Debug> std::fmt::Debug for ApplicationContext<'_, U, E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self {
            discord: _,
            interaction,
            args,
            has_sent_initial_response,
            framework: _,
            command: _,
            data,
        } = self;

        f.debug_struct("ApplicationContext")
            .field("discord", &"<serenity Context>")
            .field("interaction", interaction)
            .field("args", args)
            .field("has_sent_initial_response", has_sent_initial_response)
            .field("framework", &"<poise Framework>")
            .field("command", &"<poise Command>")
            .field("data", data)
            .finish()
    }
}

impl<U, E> ApplicationContext<'_, U, E> {
    /// See [`crate::Context::defer()`]
    pub async fn defer_response(&self, ephemeral: bool) -> Result<(), serenity::Error> {
        let interaction = match self.interaction {
            ApplicationCommandOrAutocompleteInteraction::ApplicationCommand(x) => x,
            ApplicationCommandOrAutocompleteInteraction::Autocomplete(_) => return Ok(()),
        };

        let mut flags = serenity::InteractionApplicationCommandCallbackDataFlags::empty();
        if ephemeral {
            flags |= serenity::InteractionApplicationCommandCallbackDataFlags::EPHEMERAL;
        }

        if !self
            .has_sent_initial_response
            .load(std::sync::atomic::Ordering::SeqCst)
        {
            interaction
                .create_interaction_response(self.discord, |f| {
                    f.kind(serenity::InteractionResponseType::DeferredChannelMessageWithSource)
                        .interaction_response_data(|b| b.flags(flags))
                })
                .await?;
            self.has_sent_initial_response
                .store(true, std::sync::atomic::Ordering::SeqCst);
        }
        Ok(())
    }
}

/// Possible actions that a context menu entry can have
pub enum ContextMenuCommandAction<U, E> {
    /// Context menu entry on a user
    User(
        fn(
            ApplicationContext<'_, U, E>,
            serenity::User,
        ) -> BoxFuture<'_, Result<(), crate::FrameworkError<'_, U, E>>>,
    ),
    /// Context menu entry on a message
    Message(
        fn(
            ApplicationContext<'_, U, E>,
            serenity::Message,
        ) -> BoxFuture<'_, Result<(), crate::FrameworkError<'_, U, E>>>,
    ),
}
impl<U, E> Copy for ContextMenuCommandAction<U, E> {}
impl<U, E> Clone for ContextMenuCommandAction<U, E> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<U, E> std::fmt::Debug for ContextMenuCommandAction<U, E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::User(x) => f.debug_tuple("User").field(&(x as *const ())).finish(),
            Self::Message(x) => f.debug_tuple("Message").field(&(x as *const ())).finish(),
        }
    }
}

/// A single parameter of a [`crate::Command`]
#[derive(Clone)]
pub struct CommandParameter<U, E> {
    /// Name of this command parameter
    pub name: &'static str,
    /// Description of the command. Required for slash commands
    pub description: Option<&'static str>,
    /// `true` is this parameter is required, `false` if it's optional or variadic
    pub required: bool,
    /// If this parameter is a channel, users can only enter these channel types in a slash command
    ///
    /// Prefix commands are currently unaffected by this
    pub channel_types: Option<Vec<serenity::ChannelType>>,
    /// Closure that sets this parameter's type and min/max value in the given builder
    ///
    /// For example a u32 [`CommandParameter`] would store this as the [`Self::type_setter`]:
    /// ```rust
    /// # use poise::serenity_prelude as serenity;
    /// # let _: fn(&mut serenity::CreateApplicationCommandOption) -> &mut serenity::CreateApplicationCommandOption =
    /// |b| b.kind(serenity::ApplicationCommandOptionType::Integer).min_int_value(0).max_int_value(u32::MAX)
    /// # ;
    /// ```
    pub type_setter: Option<fn(&mut serenity::CreateApplicationCommandOption)>,
    /// Optionally, a callback that is invoked on autocomplete interactions. This closure should
    /// extract the partial argument from the given JSON value and generate the autocomplete
    /// response which contains the list of autocomplete suggestions.
    pub autocomplete_callback: Option<
        for<'a> fn(
            crate::ApplicationContext<'a, U, E>,
            &'a serenity::json::Value,
        ) -> BoxFuture<
            'a,
            Result<serenity::CreateAutocompleteResponse, crate::SlashArgError>,
        >,
    >,
}

impl<U, E> CommandParameter<U, E> {
    /// Generates a slash command parameter builder from this [`CommandParameter`] instance. This
    /// can be used to register the command on Discord's servers
    pub fn create_as_slash_command_option(
        &self,
    ) -> Option<serenity::CreateApplicationCommandOption> {
        let mut builder = serenity::CreateApplicationCommandOption::default();
        builder
            .required(self.required)
            .name(self.name)
            .description(self.description?)
            .set_autocomplete(self.autocomplete_callback.is_some());
        if let Some(channel_types) = &self.channel_types {
            builder.channel_types(channel_types);
        }
        (self.type_setter?)(&mut builder);
        Some(builder)
    }
}

impl<U, E> std::fmt::Debug for CommandParameter<U, E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self {
            name,
            description,
            required,
            channel_types,
            type_setter,
            autocomplete_callback,
        } = self;

        f.debug_struct("CommandParameter")
            .field("name", name)
            .field("description", description)
            .field("required", required)
            .field("channel_types", channel_types)
            .field("type_setter", &type_setter.map(|f| f as *const ()))
            .field(
                "autocomplete_callback",
                &autocomplete_callback.map(|f| f as *const ()),
            )
            .finish()
    }
}
