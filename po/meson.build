i18n.gettext(gettext_package, preset: 'glib')

# The Rust parser skips gettext calls inside glib::clone!; this maintainer target
# also performs the earlier C-parser pass so callback feedback remains translated.
run_target(
  'momentum-update-translations',
  command: [find_program('python3'), files('../build-aux/update-translations.py')],
)
