# Contributing to `bevy_audio`

This document highlights documents some general explanation and guidelines for
contributing code to this crate. It assumes knowledge of programming, but not
necessarily of audio programming specifically. It lays out rules to follow, on
top of the general programming and contribution guidelines of Bevy, that are of
particular interest for performance reasons.

This section applies to the equivalent in abstraction level to working with
nodes in the render graph, and not manipulating entities with meshes and
materials.

Note that these guidelines are general to any audio programming application, and
not just Bevy.

## Fundamentals of working with audio

### A brief introduction to digital audio signals

Audio signals, when working within a computer, are digital streams of audio
samples (historically with different types, but nowadays the values are 32-bit
floats), taken at regular intervals of each other.

How often this sampling is done is determined by the **sample rate** parameter.
This parameter is available to the users in OS settings, as well as some
applications.

The sample rate directly determines the spectrum of audio frequencies that will
be representable by the system. That limit sits at half the sample rate, meaning
that any sound with frequencies higher than that will introduce artifacts.

If you want to learn more, read about the **Nyquist sampling theorem** and
**Frequency aliasing**.

### How the computer interfaces with the sound card

When requesting for audio input or output, the OS creates a special
high-priority thread whose task it is to take in the input audio stream, and/or
produce the output stream. The audio driver passes an audio buffer that you read
from (for input) or write to (for output). The size of that buffer is also a
parameter that is configured when opening an audio stream with the sound card,
and is sometimes reflected in application settings.

Typical values for buffer size and sample rate are 512 samples at a sample rate
of 48 kHz. This means that for every 512 samples of audio the driver is going to
send to the sound card the output callback function is run in this high-priority
audio thread.  Every second, as dictated by the sample rate, the sound card
needs 48 000 samples of audio data. This means that we can expect the callback
function to be run every `512/(48000 Hz)` or 10.666... ms.

This figure is also the latency of the audio engine, that is, how much time it
takes between a user interaction and hearing the effects out the speakers.
Therefore, there is a "tug of war" between decreasing the buffer size for
latency reasons, and increasing it for performance reasons.  The threshold for
instantaneity in audio is around 15 ms, which is why 512 is a good value for
interactive applications.

### Real-time programming

The parts of the code running in the audio thread have exactly
`buffer_size/samplerate` seconds to complete, beyond which the audio driver
outputs silence (or worse, the previous buffer output, or garbage data), which
the user perceives as a glitch and severely deteriorates the quality of the
audio output of the engine. It is therefore critical to work with code that is
guaranteed to finish in that time.

One step to achieving this is making sure that all machines across the spectrum
of supported CPUs can reliably perform the computations needed for the game in
that amount of time, and play around with the buffer size to find the best
compromise between latency and performance. Another is to conditionally enable
certain effects for more powerful CPUs, when that is possible.

But the main step is to write code to run in the audio thread following
real-time programming guidelines.  Real-time programming is a set of constraints
on code and structures that guarantees the code completes at some point, ie. it
cannot be stuck in an infinite loop nor can it trigger a deadlock situation.

Practically, the main components of real-time programming are about using
wait-free and lock-free structures. Examples of things that are *not* correct in
real-time programming are:

- Allocating anything on the heap (that is, no direct or indirect creation of a
`Vec`, `Box`, or any standard collection, as they are not designed with
real-time programming in mind)

- Locking a mutex - Generally, any kind of system call gives the OS the
opportunity to pause the thread, which is an unbounded operation as we don't
know how long the thread is going to be paused for

- Waiting by looping until some condition is met (also called a spinloop or a
spinlock)

Writing wait-free and lock-free structures is a hard task, and difficult to get
correct; however many structures already exists, and can be directly used. There
are crates for most replacements of standard collections.

### Where in the code should real-time programming principles be applied?

Any code that is directly or indirectly called by audio threads, needs to be
real-time safe.

For the Bevy engine, that is:

- In the callback of `cpal::Stream::build_input_stream` and
`cpal::Stream::build_output_stream`, and all functions called from them

- In implementations of the [`Source`] trait, and all functions called from it

Code that is run in Bevy systems do not need to be real-time safe, as they are
not run in the audio thread, but in the main game loop thread.

## Communication with the audio thread

To be able to to anything useful with audio, the thread has to be able to
communicate with the rest of the system, ie. update parameters, send/receive
audio data, etc., and all of that needs to be done within the constraints of
real-time programming, of course.

### Audio parameters

In most cases, audio parameters can be represented by an atomic floating point
value, where the game loop updates the parameter, and it gets picked up when
processing the next buffer. The downside to this approach is that the audio only
changes once per audio callback, and results in a noticeable "stair-step "
motion of the parameter. The latter can be mitigated by "smoothing" the change
over time, using a tween or linear/exponential smoothing.

Precise timing for non-interactive events (ie. on the beat) need to be setup
using a clock backed by the audio driver -- that is, counting the number of
samples processed, and deriving the time elapsed by diving by the sample rate to
get the number of seconds elapsed. The precise sample at which the parameter
needs to be changed can then be computed.

Both interactive and precise events are hard to do, and need very low latency
(ie. 64 or 128 samples for ~2 ms of latency). It is fundamentally impossible to
react to user event the very moment it is registered.

### Audio data

Audio data is generally transferred between threads with circular buffers, as
they are simple to implement, fast enough for 99% of use-cases, and are both
wait-free and lock-free. The only difficulty in using circular buffers is how
big they should be; however even going for 1 s of audio costs ~50 kB of memory,
which is small enough to not be noticeable even with potentially 100s of those
buffers.

## Additional resources for audio programming

More in-depth article about audio programming:
<http://www.rossbencina.com/code/real-time-audio-programming-101-time-waits-for-nothing>

Awesome Audio DSP: <https://github.com/BillyDM/awesome-audio-dsp>
