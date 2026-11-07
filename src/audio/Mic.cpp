#include "Mic.h"

#include <cmath>

#include <QtEndian>

namespace {
// shortcut: one fixed gate for every microphone; a quiet laptop mic may need
// it lowered, a noisy room raised. Calibrate per device when a second user
// reports it, not before.
constexpr double SpeechRms = 0.020;
constexpr int FramesToStart = 3;   // ~60 ms of sound before it counts
constexpr int SilenceMs = 900;     // a normal sentence pause, not a full stop
} // namespace

Mic::Mic(QObject *parent)
    : QObject(parent)
{
    m_silence.setSingleShot(true);
    m_silence.setInterval(SilenceMs);
    connect(&m_silence, &QTimer::timeout, this, &Mic::onSilence);
    connect(&m_capture, &AudioCapture::frameReady, this, &Mic::onFrame);
}

void Mic::start()
{
    if (!m_muted)
        m_capture.start();
    const bool now = m_capture.isRunning();
    if (now != m_available) {
        m_available = now;
        emit availableChanged();
    }
}

void Mic::stop()
{
    m_capture.stop();
    m_silence.stop();
    if (m_speech) {
        m_speech = false;
        emit speechChanged();
    }
    if (m_available) {
        m_available = false;
        emit availableChanged();
    }
}

void Mic::setMuted(bool muted)
{
    m_muted = muted;
    if (muted)
        stop();
    else
        start();
}

void Mic::onFrame(const QByteArray &pcm)
{
    const auto *samples = reinterpret_cast<const qint16 *>(pcm.constData());
    const int count = int(pcm.size()) / int(sizeof(qint16));
    if (count == 0)
        return;

    // qFromLittleEndian is not needed on x86/ARM little-endian hosts, but the
    // device path must not assume it.
    double sum = 0.0;
    for (int i = 0; i < count; ++i) {
        const double v = double(qFromLittleEndian<qint16>(samples + i)) / 32768.0;
        sum += v * v;
    }
    const double rms = std::sqrt(sum / count);

    if (rms < SpeechRms) {
        // Start once, on the first quiet frame: restarting it on every frame
        // would keep pushing the end back and the turn would never close.
        if (m_speech && !m_silence.isActive())
            m_silence.start();
        else
            m_loudFrames = 0;
        return;
    }

    m_silence.stop();
    if (m_speech)
        return;

    if (++m_loudFrames < FramesToStart)
        return;

    m_loudFrames = 0;
    m_speech = true;
    emit speechChanged();
    emit speechStarted();
}

void Mic::onSilence()
{
    m_loudFrames = 0;
    if (!m_speech)
        return;
    m_speech = false;
    emit speechChanged();
    emit speechEnded();
}
