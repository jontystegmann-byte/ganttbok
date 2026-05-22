/// Template renderer for chaser messages. Substitutes `{task}`, `{days}`, `{contact_name}`,
/// `{job_name}` in a template string with values from the supplied context.
///
/// Unknown placeholders are left intact rather than erroring — this lets the user write
/// templates with future placeholders we add later without crashing.

pub struct TemplateContext<'a> {
    pub task_name: &'a str,
    pub job_name: &'a str,
    pub contact_name: &'a str,
    pub days: i64,
}

pub fn render(template: &str, ctx: &TemplateContext) -> String {
    template
        .replace("{task}", ctx.task_name)
        .replace("{job_name}", ctx.job_name)
        .replace("{contact_name}", ctx.contact_name)
        .replace("{days}", &ctx.days.to_string())
        .replace("{days_abs}", &ctx.days.abs().to_string())
}

/// The three default templates shipped with the app. Used when the user hasn't customised them.
pub const DEFAULT_MANUAL: &str = "Update me on *{task}* — what's the latest?";
pub const DEFAULT_APPROACHING: &str = "*{task}* deadline is in {days} days — still on track?";
pub const DEFAULT_OVERDUE: &str = "*{task}* was due {days_abs} days ago — what's the blocker?";

/// The set of valid chaser template keys accepted by `apply_add_chaser`.
/// Matches the hard-coded keys handled in `commands::chaser::send_chaser`.
pub const VALID_CHASER_TEMPLATE_KEYS: &[&str] = &["manual", "approaching", "overdue"];

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> TemplateContext<'static> {
        TemplateContext {
            task_name: "Install windows",
            job_name: "Noordhoek House",
            contact_name: "Caleb",
            days: 3,
        }
    }

    #[test]
    fn substitutes_task_placeholder() {
        let out = render("Update me on {task}", &ctx());
        assert_eq!(out, "Update me on Install windows");
    }

    #[test]
    fn substitutes_days_and_abs() {
        let mut c = ctx();
        c.days = -5;
        let out = render("{task} was due {days_abs} days ago", &c);
        assert_eq!(out, "Install windows was due 5 days ago");
    }

    #[test]
    fn unknown_placeholder_left_intact() {
        let out = render("Hello {nobody}", &ctx());
        assert_eq!(out, "Hello {nobody}");
    }

    #[test]
    fn all_default_templates_render() {
        let c = ctx();
        for t in [DEFAULT_MANUAL, DEFAULT_APPROACHING, DEFAULT_OVERDUE] {
            let out = render(t, &c);
            assert!(!out.contains("{task}") && !out.contains("{days}"),
                    "template still has placeholders: {out}");
        }
    }
}
