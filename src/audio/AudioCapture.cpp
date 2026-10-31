#include "audio/AudioCapture.h"

#include <QIODevice>

#ifdef NEWTON_HAVE_AUDIO_SOURCE
#include <QAudioFormat>
#include <QMediaDevices>
#endif

AudioCapture::AudioCapture(QObject *parent)
    : QObject(parent)
{
    m_ring.reserve(MaxBufferedBytes);
}

AudioCapture::~AudioCapture()
{
    stop();
}

#ifdef NEWTON_HAVE_AUDIO_SOURCE
QList<QAudioDevice> AudioCapture::devices() const
{
    return QMediaDevices::audioInputs();
}
#endif

void AudioCapture::start()
{
    // A second start() must not open a second source.
    if (m_running)
        return;
    m_running = openSource();
}

void AudioCapture::stop()
{
    if (!m_running)
        return;
    closeSource();
    m_running = false;
}

bool AudioCapture::openSource()
{
#ifdef NEWTON_HAVE_AUDIO_SOURCE
    const QAudioDevice device = QMediaDevices::defaultAudioInput();
    if (device.isNull())
        return false;

    QAudioFormat format;
    format.setSampleRate(SampleRate);
    format.setChannelCount(Channels);
    // shortcut: Qt's Int16 is host byte order, so this is little-endian only on
    // the little-endian targets we ship. Upgrade if a big-endian build appears.
    format.setSampleFormat(QAudioFormat::Int16);

    // Resampling belongs to the harness, not here. A device that cannot give us
    // the target format is declined rather than silently relabelled.
    if (!device.isFormatSupported(format))
        return false;

    m_source = new QAudioSource(device, format, this);
    connect(m_source, &QAudioSource::stateChanged, this, &AudioCapture::onStateChanged);
    m_io = m_source->start();
    if (!m_io) {
        closeSource();
        return false;
    }
    connect(m_io, &QIODevice::readyRead, this, &AudioCapture::onReadyRead);
    return true;
#else
    return false;
#endif
}

void AudioCapture::closeSource()
{
#ifdef NEWTON_HAVE_AUDIO_SOURCE
    if (m_io) {
        disconnect(m_io, nullptr, this, nullptr);
        m_io = nullptr;
    }
    if (m_source) {
        disconnect(m_source, nullptr, this, nullptr);
        m_source->stop();
        // deleteLater, because this can run from inside m_source's own signal.
        m_source->deleteLater();
        m_source = nullptr;
    }
#endif
}

void AudioCapture::pushFrameForTest(const QByteArray &pcm)
{
    if (m_running)
        ingest(pcm);
}

void AudioCapture::ingest(const QByteArray &pcm)
{
    if (pcm.isEmpty())
        return;

    // Bounded: old audio is dropped, the ring is never resized past the cap.
    m_ring.append(pcm);
    if (m_ring.size() > MaxBufferedBytes)
        m_ring.remove(0, int(m_ring.size()) - MaxBufferedBytes);

    emit frameReady(pcm);
}

#ifdef NEWTON_HAVE_AUDIO_SOURCE
void AudioCapture::onReadyRead()
{
    if (m_io)
        ingest(m_io->readAll());
}

void AudioCapture::onStateChanged()
{
    if (!m_running || !m_source || m_source->state() != QAudio::StoppedState)
        return;

    // The device disappeared. Say nothing, write nothing, and let a later
    // start() open whatever device is there then.
    stop();
}
#endif
