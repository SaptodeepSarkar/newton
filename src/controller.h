#pragma once

#include <QObject>
#include <QTimer>

#include "audio/WakeModel.h"
#include "orbstate.h"

class Mic;
class Speaker;

// The device's hands: it opens the microphone when asked to listen, follows the
// voice, speaks the reply, and puts the orb back to rest. No model is involved
// yet, so the reply is a fixed line - but the timing and the states are real.
class Controller : public QObject
{
    Q_OBJECT
    Q_PROPERTY(double level READ level NOTIFY levelChanged)
    Q_PROPERTY(bool available READ available NOTIFY availableChanged)
    Q_PROPERTY(QString voice READ voice NOTIFY voiceChanged)
    Q_PROPERTY(bool wakeReady READ wakeReady NOTIFY wakeReadyChanged)

public:
    // boot=false keeps the orb in Booting so --shots and --check can walk it.
    explicit Controller(OrbState *orb, bool boot = true, QObject *parent = nullptr);

    double level() const { return m_level; }
    bool available() const;
    QString voice() const;
    bool wakeReady() const { return m_wake.isReady(); }
    // Where the enrolled templates live; shared with the enrolment mode.
    static QString wakeModelPath();

    // Click the puck, or press Space: listen for one sentence.
    Q_INVOKABLE void talk();
    // Dev seam: prove the voice path without a conversation.
    Q_INVOKABLE void say(const QString &text);

signals:
    void levelChanged();
    void availableChanged();
    void voiceChanged();
    void wakeReadyChanged();

private:
    // Opens the ears and, coming from Booting, reaches StandBy first.
    void listen();
    void onSpeechStarted();
    void onSpeechEnded();
    void onSpoke();
    void onPatience();

    OrbState *m_orb = nullptr;
    Mic *m_mic = nullptr;
    Speaker *m_speaker = nullptr;
    WakeModel m_wake;
    QTimer m_patience;
    double m_level = 0.0;
};
