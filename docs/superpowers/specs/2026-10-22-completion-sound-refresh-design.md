# Quiet completion sounds and auditioning

Status: approved by the user on 2026-10-22; all four native platforms. Part of `task/native-experience-overhaul`.
Related: [Settings and About](2026-10-22-settings-discovery-about-design.md).

## Findings and intended behavior

User reports completion sounds are too loud and dislikes the current catalog.
Current source uses AudioServices system sounds on Apple, SoundPool with unit gain
on Android, and GtkMediaFile without explicit attenuation on Linux. Current preview
uses the saved sound and is gated on completion sounds being enabled. These explain
missing volume control and preview-before-selection; excessive perceived loudness
still requires native listening, not just a waveform measurement.

Provide ten new short, warm, positive choices under Sounds & Feedback. Each named
candidate has separate native Preview and selection controls. Preview never saves
the candidate, enables completion feedback, completes a task, or opens a permission
prompt. Preview works even when automatic completion feedback is off. Selection
does not autoplay. Persist only deliberate selection; expose the current choice
clearly to assistive technology, with descriptive Preview labels per sound.

Use one app-level volume control, device-local, defaulting to a quiet 35%, shared
by candidate previews and actual completion sounds. It scales normalized masters
within system volume; never changes the OS volume or bypasses mute/silent policy.
The exact default is subject to listening on Mac and available mobile devices.
Expose the control and numeric accessibility value; zero is silent. Keep the global
completion toggle off by default. Changing volume does not autoplay.

Normalize masters conservatively, with smooth attacks/releases, no clipping or
sharp transients. Draft target: peak at most -18 dBFS and RMS at most -32 dBFS,
duration 0.18–0.45 seconds. These are digital level targets, not a guarantee of
physical loudness on unknown speakers/headphones. Check consistency by listening.
Preview and completion use the same master, gain and routing. Only one clip plays
at a time; a new preview replaces the old one, with no queued burst after loading.
Cancel pending/current previews on leaving the picker or app. Automatic completion
continues to run only for successful interactive completion, not undo, sync or
failed changes. Rapid completions must not sum into a loud overlapping burst.

Native players own audio: Apple needs a gain-capable player, with iOS ambient/mixing
and silent-switch behavior preserved; Android keeps sonification SoundPool with
bounded gain and native mute policy; GTK sets bounded MediaStream volume and
replaces/releases prior playback. No new audio engine/dependency or background
audio entitlement. Errors remain nonfatal and must never prevent task completion.

## Ten original audition drafts

1. Soft Knock — rounded wooden tap, 0.22 s.
2. Little Lift — gentle two-note rise, 0.32 s.
3. Felt Finish — muted consonant chord, 0.28 s.
4. Warm Thumb — soft thumb-piano-like pluck, 0.30 s.
5. Gentle Pair — two light wooden notes, 0.30 s.
6. Small Bloom — brief spreading major chord, 0.40 s.
7. Sunny Tap — bright but softened tap, 0.24 s.
8. Pocket Harp — three light plucks, 0.37 s.
9. Easy Landing — calm two-note settling phrase, 0.36 s.
10. Quiet Cheer — tiny ascending consonant phrase, 0.43 s.

Drafts are original procedural resonator synthesis, not recordings of instruments
or third-party samples. Audition assets and measured SHA256/levels are in the chat's
local artifact directory, not shipping resources. Refine after listening; don't
claim creative approval or acceptable device loudness from generated files alone.
Before shipping, retain a reproducible generator, provenance, hashes and appropriate
project-compatible licensing/notices. Package byte-identical masters on all apps.

Keep established stored sound IDs readable and unknown raw values preserved. Show
a prior saved choice as a clearly labeled legacy selection until the user chooses
a new sound; do not silently replace it. Apply conservative attenuation to legacy
playback too. Extend native catalogs and the portable sound-value validation within
the existing extensible format; do not migrate task/storage/settings formats. New
IDs must round-trip without corrupting older/unknown values. Volume stays outside
settings sync/transfer unless separately approved. Default for a fresh installation
can be Little Lift after audition approval; existing choices remain unchanged.

## Verification and documentation

Add unit/state coverage for previewing unsaved candidates while automatic feedback
is off; no preference mutations on preview/cancel; save without autoplay; bounded
gain/zero; replacement and cancellation across pending asynchronous loads; successful
completion-only playback; legacy/unknown IDs and portable round-trip; exact catalog
and master-byte parity across packages. Asset checks cover duration, PCM shape,
peak/RMS bounds, silence, fades, hashes and missing resources. Avoid new UI suites.

Record the reported loudness issue in BUGS (user report versus local reproduction),
extend FEATURES for audition-before-selection and volume, and update PROGRESS with
actual checks, platform builds, native listening evidence and remaining gaps. Do
not mark all platforms verified from host tests or measurements alone.

## Native references

- Apple AVAudioPlayer volume and AVAudioSession ambient category documentation.
- Android SoundPool play gain and AudioAttributes sonification documentation.
- GTK MediaStream volume property documentation.
