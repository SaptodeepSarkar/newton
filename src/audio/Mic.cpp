#include "Mic.h"

#include <algorithm>
#include <cmath>

#include <QtEndian>

namespace {
// The gate is relative to the room, not to a fixed number: this machine idles
// at 0.12 RMS, which any absolute threshold would call speech all day.
constexpr int FloorFrames = 100;    // ~2 s of history
constexpr int WarmupFrames = 40;    // learn the room before deciding anything
constexpr double MinLoud = 0.045;   // never trigger on a whisper of noise
constexpr double FloorRatio = 1.6;  // a voice sits well above the room
constexpr int FramesToStart = 15;   // ~300 ms of sound before it counts
constexpr int SilenceMs = 900;      // a normal sentence pause, not a full stop
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

    m_history.append(rms);
    if (m_history.size() > FloorFrames)
        m_history.removeFirst();

    QVector<double> sorted = m_history;
    std::sort(sorted.begin(), sorted.end());
    const double floor = sorted.at(sorted.size() / 2);
    const double gate = qMax(MinLoud, floor * FloorRatio);
    const bool warm = m_history.size() < WarmupFrames;

    if (warm || rms < gate) {
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
