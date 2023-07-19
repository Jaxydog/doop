/// Responds to an interaction event.
#[macro_export]
macro_rules! respond {
    ($ctx:expr, { $($args:tt)* }) => {
        $crate::respond! {
            @build($ctx.client(), $ctx.event.id, &$ctx.event.token) { $($args)* }
        }
    };
    ($http:expr, $int:expr, { $($args:tt)* }) => {
        $crate::respond! {
            @build($http.interaction($int.application_id), $int.id, &$int.token) { $($args)* }
        }
    };
    (@build($client:expr, $id:expr, $token:expr) {
        KIND = $kind:ident,
        $( MENTIONS = { $( $body:tt ),* $(,)? }, )?
        $( ATTACHMENTS = [ $( $attachment:expr ),* $(,)? ], )?
        $( CHOICES = $choices:expr, )?
        $( CHOICES = [ $( $choice:expr ),* $(,)? ], )?
        $( COMPONENTS = $components:expr, )?
        $( COMPONENTS = [ $( $component:expr ),* $(,)? ], )?
        $( CONTENT = $content:expr, )?
        $( CUSTOM_ID = $custom_id:expr, )?
        $( EMBEDS = [ $( $embed:expr ),* $(,)? ], )?
        $( FLAGS = [ $( $flag:ident ),* $(,)? ], )?
        $( TITLE = $title:expr, )?
        $( TTS = $tts:literal, )?
    }) => {
        $client.create_response($id, $token, &::twilight_model::http::interaction::InteractionResponse {
            kind: ::twilight_model::http::interaction::InteractionResponseType::$kind,
            data: Some(::twilight_util::builder::InteractionResponseDataBuilder::new()
                $(.allowed_mentions(::twilight_model::channel::message::AllowedMentions { $($body)* }))?
                $(.attachments([$($attachment),*]))?
                $(.choices($choices))?
                $(.choices([$($choice),*]))?
                $(.components($components))?
                $(.components([$($component),*]))?
                $(.content($content))?
                $(.custom_id($custom_id))?
                $(.embeds([$($embed),*]))?
                $(.flags(::twilight_model::channel::message::MessageFlags::empty()$(.union(::twilight_model::channel::message::MessageFlags::$flag))*))?
                $(.title($title))?
                $(.tts($tts))?
                .build()
            )
        })
    };
}

/// Follows-up an interaction event response.
#[macro_export]
macro_rules! followup {
    ($ctx:expr, { $($args:tt)* }) => {
        $crate::followup! {
            @build($ctx.client(), &$ctx.event.token) { $($args)* }
        }
    };
    ($http:expr, $int:expr, { $($args:tt)* }) => {
        $crate::followup! {
            @build($http.interaction($int.application_id), &$int.token) { $($args)* }
        }
    };
    (@build($client:expr, $token:expr) {
        $( MENTIONS = { $( $body:tt ),* $(,)? }, )?
        $( ATTACHED = [ $( $attachment:expr ),* $(,)? ], )?
        $( COMPONENTS = $components:expr, )?
        $( COMPONENTS = [ $( $component:expr ),* $(,)? ], )?
        $( CONTENT = $content:expr, )?
        $( EMBEDS = [ $( $embed:expr ),* $(,)? ], )?
        $( FLAGS = [ $( $flag:ident ),* $(,)? ], )?
        $( TTS = $tts:literal, )?
    }) => {
        $client.create_followup(&$token)
        $(.allowed_mentions(::twilight_model::channel::message::AllowedMentions { $($body)* }))?
        $(.attachments([$($attachment),*])?)?
        $(.components($components)?)?
        $(.components(&[$($component),*])?)?
        $(.content($content)?)?
        $(.embeds(&[$($embed),*])?)?
        $(.flags(::twilight_model::channel::message::MessageFlags::empty()$(.union(::twilight_model::channel::message::MessageFlags::$flag))*))?
        $(.tts($tts))?
    };
}
