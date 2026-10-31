#pragma once

#include <QByteArray>
#include <QList>
#include <QObject>

// Multimedia is optional: a headless box without it still builds the library,
// the ring buffer and the tests. The device path is the only part dropped.
#if defined(NEWTON_HAS_MULTIMEDIA) && __has_include(<QAudioDevice>)
#include <QAudioDevice>
#include <QAudioSource>
#define NEWTON_HAVE_AUDIO_SOURCE 1
#endif

// Microphone capture at 16 kHz, mono, 16-bit signed little-endian. Every frame
// goes out on frameReady and into a ring buffer capped at three seconds.
// Nothing here writes to disk: no temp file, no log, no dump.
class AudioCapture : public QObject
{
    Q_OBJECT

public:
    static constexpr int SampleRate = 16000;
    static constexpr int Channels = 1;
    static constexpr int BytesPerSample = 2;
    static constexpr int MaxBufferedSamples = 3 * SampleRate; // 48000
    static constexpr int MaxBufferedBytes = MaxBufferedSamples * BytesPerSample;

    explicit AudioCapture(QObject *parent = nullptr);
    ~AudioCapture() override;

    // Both are no-ops when already in that state. start() stays stopped when
    // there is no usable device; it can be tried again later.
    void start();
    void stop();
    bool isRunning() const { return m_running; }

#ifdef NEWTON_HAVE_AUDIO_SOURCE
    QList<QAudioDevice> devices() const;
#endif

    // What the ring holds now, never more than MaxBuffered*.
    int bufferedSamples() const { return int(m_ring.size()) / BytesPerSample; }
    int bufferedBytes() const { return int(m_ring.size()); }

    // Test-only: feeds a frame down the same path a real device read takes, so
    // the ring and the signal are testable without a microphone. Ignored while
    // stopped, exactly like a frame from a device that went away.
    void pushFrameForTest(const QByteArray &pcm);

signals:
    void frameReady(const QByteArray &pcm);

protected:
    // Test-only seams. The defaults open and close the real default input.
    // Override both in a test so no device is ever touched.
    virtual bool openSource();
    virtual void closeSource();

private:
    void ingest(const QByteArray &pcm);

#ifdef NEWTON_HAVE_AUDIO_SOURCE
    void onReadyRead();
    void onStateChanged();
    QAudioSource *m_source = nullptr;
    QIODevice *m_io = nullptr;
#endif

    QByteArray m_ring;
    bool m_running = false;
};
