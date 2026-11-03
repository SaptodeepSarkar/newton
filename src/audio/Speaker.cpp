#include "Speaker.h"

#include <QFile>
#include <QProcess>
#include <QStandardPaths>
#include <QtEndian>
#include <QtMath>

#if defined(NEWTON_HAS_MULTIMEDIA) && __has_include(<QAudioSink>)
#include <QAudioFormat>
#include <QAudioSink>
#include <QMediaDevices>
#define NEWTON_HAVE_AUDIO_SINK 1
#endif

namespace {
constexpr int ChimeRate = 16000;

// A rising two-tone, 120 ms each, faded so it never clicks.
QByteArray chimeBytes()
{
    const int rate = ChimeRate;
    const int toneSamples = rate * 120 / 1000;
    QByteArray out;
    out.reserve(toneSamples * 2 * 2 * 2);
    for (int tone = 0; tone < 2; ++tone) {
        const double hz = tone == 0 ? 660.0 : 880.0;
        for (int i = 0; i < toneSamples; ++i) {
            const double t = double(i) / rate;
            // 8 ms fade in and out per tone: no transient through cheap speakers.
            const double edge = 8.0 / 120.0;
            const double pos = double(i) / toneSamples;
            const double fade = qMin(1.0, qMin(pos / edge, (1.0 - pos) / edge));
            const double v = qSin(2.0 * M_PI * hz * t) * 0.32 * qMax(0.0, fade);
            const auto s = qint16(v * 32767.0);
            uchar bytes[2];
            qToLittleEndian<qint16>(s, bytes);
            out.append(reinterpret_cast<const char *>(bytes), 2);
        }
    }
    return out;
}
} // namespace

Speaker::Speaker(QObject *parent)
    : QObject(parent)
{
    m_hasEngine = !QStandardPaths::findExecutable(QStringLiteral("espeak-ng")).isEmpty();
    m_tick.setInterval(33);
    connect(&m_tick, &QTimer::timeout, this, &Speaker::step);
}

Speaker::~Speaker()
{
    stop();
}

QString Speaker::engine() const
{
    return m_hasEngine ? QStringLiteral("espeak-ng") : QStringLiteral("chime");
}

Speaker::Pcm Speaker::spoken(const QString &text) const
{
    Pcm pcm;
    if (!m_hasEngine)
        return pcm;

    QProcess proc;
    proc.start(QStringLiteral("espeak-ng"),
               {QStringLiteral("--stdout"), QStringLiteral("-v"),
                QStringLiteral("en+f3"), QStringLiteral("-s"),
                QStringLiteral("165"), text});
    if (!proc.waitForFinished(4000) || proc.exitCode() != 0)
        return pcm;

    const QByteArray wav = proc.readAllStandardOutput();
    // Canonical RIFF: locate fmt and data rather than assuming fixed offsets.
    int fmtAt = wav.indexOf("fmt ");
    int dataAt = wav.indexOf("data");
    if (fmtAt < 0 || dataAt < 0 || dataAt + 8 > wav.size())
        return pcm;

    const auto *fmt = reinterpret_cast<const uchar *>(wav.constData() + fmtAt + 8);
    pcm.channels = int(qFromLittleEndian<quint16>(fmt + 2));
    pcm.sampleRate = int(qFromLittleEndian<quint32>(fmt + 4));

    const auto *len = reinterpret_cast<const uchar *>(wav.constData() + dataAt + 4);
    const int size = int(qFromLittleEndian<quint32>(len));
    pcm.bytes = wav.mid(dataAt + 8, qMin(size, int(wav.size()) - dataAt - 8));
    if (pcm.channels < 1 || pcm.sampleRate < 8000)
        pcm = Pcm{};
    return pcm;
}

Speaker::Pcm Speaker::chime() const
{
    Pcm pcm;
    pcm.bytes = chimeBytes();
    pcm.sampleRate = ChimeRate;
    return pcm;
}

void Speaker::say(const QString &text)
{
    stop();
    Pcm pcm = spoken(text);
    if (pcm.bytes.isEmpty())
        pcm = chime();
    play(pcm);
}

void Speaker::play(const Pcm &pcm)
{
    // The envelope is derived from the samples themselves, so the ring moves
    // with what is actually coming out of the speaker, not with a guess.
    const int frame = qMax(1, pcm.sampleRate * 30 / 1000) * qMax(1, pcm.channels);
    const auto *samples = reinterpret_cast<const qint16 *>(pcm.bytes.constData());
    const int total = int(pcm.bytes.size()) / 2;
    m_envelope.clear();
    for (int start = 0; start + frame <= total; start += frame) {
        double sum = 0.0;
        for (int i = 0; i < frame; ++i) {
            const double v = double(samples[start + i]) / 32768.0;
            sum += v * v;
        }
        m_envelope.append(qMin(1.0, std::sqrt(sum / frame) * 3.0));
    }
    m_at = 0;

#ifdef NEWTON_HAVE_AUDIO_SINK
    QAudioFormat format;
    format.setSampleRate(pcm.sampleRate);
    format.setChannelCount(pcm.channels);
    format.setSampleFormat(QAudioFormat::Int16);
    if (!QMediaDevices::defaultAudioOutput().isFormatSupported(format)) {
        fprintf(stderr, "SPEAKER: device rejects %d Hz %d ch, animating only\n",
                pcm.sampleRate, pcm.channels);
        // Still animate: the words exist even when the device cannot take them.
        m_speaking = true;
        emit speakingChanged();
        m_tick.start();
        return;
    }
    m_buffer.setData(pcm.bytes);
    m_buffer.open(QIODevice::ReadOnly);
    m_sink = new QAudioSink(format, this);
    m_sink->start(&m_buffer);
#endif

    m_speaking = true;
    emit speakingChanged();
    m_tick.start();
}

void Speaker::step()
{
    if (m_at >= m_envelope.size()) {
        end();
        return;
    }
    m_level = m_envelope.at(m_at++);
    emit levelChanged();
}

void Speaker::end()
{
    m_tick.stop();
    m_level = 0.0;
    emit levelChanged();
#ifdef NEWTON_HAVE_AUDIO_SINK
    if (m_sink) {
        m_sink->stop();
        m_sink->deleteLater();
        m_sink = nullptr;
    }
#endif
    m_buffer.close();
    if (m_speaking) {
        m_speaking = false;
        emit speakingChanged();
    }
    emit finished();
}

void Speaker::stop()
{
    if (!m_speaking) {
        m_envelope.clear();
        m_at = 0;
        return;
    }
    end();
}
