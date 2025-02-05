Emit a warning message to the end user.

A warning should be used to emit a message to the end user about a potential problem.

Multiple warnings can be emitted in sequence. The buildpack author should take care not to
overwhelm the end user with unnecessary warnings.

When emitting a warning, describe the problem to the user, if possible, and tell them how
to fix it or where to look next.

Warnings should often come with some disabling mechanism, if possible. If the user can turn
off the warning, that information should be included in the warning message. If you're
confident that the user should not be able to turn off a warning, consider using a
[`Print::error`] instead.

Warnings will be output in a multi-line paragraph style. A warning can be emitted from any
state except for [`state::Header`].
