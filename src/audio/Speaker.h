#pragma once

#include <QBuffer>
#include <QByteArray>
#include <QObject>
#include <QTimer>
#include <QVector>

class QAudioSink;

// The voice out. Two paths, same one: a word from the local speech engine when
// the machine has one, a two-tone chime when it does not. Never silent, so the
// ring's Speaking state always has something to show.
class Speaker : public QObject
{
    Q_OBJECT
    Q_PROPERTY(double level READ level NOTIFY levelChanged)
    Q_PROPERTY(bool speaking READ speaking NOTIFY speakingChanged)
    Q_PROPERTY(QString engine READ engine CONSTANT)

public:
    explicit Speaker(QObject *parent = nullptr);
    ~Speaker() override;

    double level() const { return m_level; }
    bool speaking() const { return m_speaking; }
    // "espeak-ng" when words are possible, "chime" when only tone is.
    QString engine() const;

    // Says `text` if a local engine exists, otherwise rings the chime.
    void say(const QString &text);
    void stop();

signals:
    void levelChanged();
    void speakingChanged();
    void finished();

private:
    struct Pcm {
        QByteArray bytes;
        int sampleRate = 16000;
        int channels = 1;
    };

    Pcm spoken(const QString &text) const;
    Pcm chime() const;
    void play(const Pcm &pcm);
    void step();
    void end();

    QAudioSink *m_sink = nullptr;
    QBuffer m_buffer;
    QTimer m_tick;
    QVector<double> m_envelope;
    int m_at = 0;
    double m_level = 0.0;
    bool m_speaking = false;
    bool m_hasEngine = false;
};
