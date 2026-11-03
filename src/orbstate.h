#pragma once

#include <QObject>
#include <QString>

// The ring's state machine. Five visual states plus one "working" flag for the
// dotted phase of Listening. No Qt Quick here, so it can be tested headless.
class OrbState : public QObject
{
    Q_OBJECT
    Q_PROPERTY(QString state READ stateName NOTIFY stateChanged)
    Q_PROPERTY(bool busy READ busy NOTIFY busyChanged)
    Q_PROPERTY(QString error READ error NOTIFY errorChanged)
    Q_PROPERTY(bool muted READ muted NOTIFY mutedChanged)

public:
    enum class State { Booting, StandBy, Listening, Speaking, Error };
    Q_ENUM(State)

    explicit OrbState(QObject *parent = nullptr);

    State state() const { return m_state; }
    QString stateName() const { return name(m_state); }
    bool busy() const { return m_busy; }
    QString error() const { return m_error; }
    bool muted() const { return m_muted; }

    // False and no change when the move is not allowed.
    bool go(State next);
    void setBusy(bool busy);
    void setMuted(bool muted);
    void fail(const QString &reason);

    // Dev cycle: standby -> listening -> busy -> speaking -> error -> standby.
    Q_INVOKABLE bool advance();

    static QString name(State state);
    static bool canReach(State from, State to);

signals:
    void stateChanged();
    void busyChanged();
    void errorChanged();
    void mutedChanged();

private:
    State m_state = State::Booting;
    bool m_busy = false;
    bool m_muted = false;
    QString m_error;
};
