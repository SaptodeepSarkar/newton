#include "orbstate.h"

OrbState::OrbState(QObject *parent)
    : QObject(parent)
{
}

QString OrbState::name(State state)
{
    switch (state) {
    case State::Booting:
        return QStringLiteral("booting");
    case State::StandBy:
        return QStringLiteral("standby");
    case State::Listening:
        return QStringLiteral("listening");
    case State::Speaking:
        return QStringLiteral("speaking");
    case State::Error:
        return QStringLiteral("error");
    }
    return QStringLiteral("booting");
}

bool OrbState::canReach(State from, State to)
{
    if (from == to)
        return false;

    switch (from) {
    case State::Booting:
        return to == State::StandBy || to == State::Error;
    case State::StandBy:
        return to == State::Listening || to == State::Error;
    case State::Listening:
        return to == State::Speaking || to == State::StandBy || to == State::Error;
    case State::Speaking:
        // Listening is barge-in: the user talks over the assistant.
        return to == State::Listening || to == State::StandBy || to == State::Error;
    case State::Error:
        // One way out. The user retries; nothing else.
        return to == State::StandBy;
    }
    return false;
}

bool OrbState::go(State next)
{
    if (!canReach(m_state, next))
        return false;

    m_state = next;
    if (next != State::Listening)
        setBusy(false);

    // Leaving the error state clears the message; entering it keeps the reason.
    if (next != State::Error && !m_error.isEmpty()) {
        m_error.clear();
        emit errorChanged();
    }
    emit stateChanged();
    return true;
}

void OrbState::setBusy(bool busy)
{
    // "Working" only means anything while listening; anywhere else it is noise.
    const bool next = (m_state == State::Listening) && busy;
    if (next == m_busy)
        return;
    m_busy = next;
    emit busyChanged();
}

void OrbState::fail(const QString &reason)
{
    m_error = reason;
    emit errorChanged();
    if (m_state != State::Error)
        go(State::Error);
}

bool OrbState::advance()
{
    switch (m_state) {
    case State::Booting:
        return go(State::StandBy);
    case State::StandBy:
        return go(State::Listening);
    case State::Listening:
        if (!m_busy) {
            setBusy(true);
            return true;
        }
        return go(State::Speaking);
    case State::Speaking:
        fail(QStringLiteral("dev cycle"));
        return true;
    case State::Error:
        return go(State::StandBy);
    }
    return false;
}
