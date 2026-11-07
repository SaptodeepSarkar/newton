#pragma once

#include <QObject>
#include <QVector>
#include <QTimer>

#include "AudioCapture.h"

// Turns a stream of 16 kHz frames into the only two facts the orb needs:
// "someone is talking" and "they stopped". Energy only, no model: a real wake
// word replaces `speechStarted` later, the ring does not care which one fired.
class Mic : public QObject
{
    Q_OBJECT
    Q_PROPERTY(bool available READ available NOTIFY availableChanged)
    Q_PROPERTY(bool speech READ speech NOTIFY speechChanged)

public:
    explicit Mic(QObject *parent = nullptr);

    bool available() const { return m_available; }
    bool speech() const { return m_speech; }

    // Opens the microphone. Silent no-op when there is no device: the app still
    // runs, the orb just never reaches Listening.
    void start();
    void stop();
    void setMuted(bool muted);
    bool isRunning() const { return m_capture.isRunning(); }

signals:
    void availableChanged();
    void speechChanged();
    void speechStarted();
    void speechEnded();

private:
    void onFrame(const QByteArray &pcm);
    void onSilence();

    AudioCapture m_capture;
    QTimer m_silence;
    bool m_available = false;
    bool m_speech = false;
    bool m_muted = false;
    int m_loudFrames = 0;
    QVector<double> m_history; // ~2 s of frame energy, for the noise floor
};
