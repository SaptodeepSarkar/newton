#include "controller.h"

#include "audio/Mic.h"
#include "audio/Speaker.h"

Controller::Controller(OrbState *orb, bool boot, QObject *parent)
    : QObject(parent)
    , m_orb(orb)
    , m_mic(new Mic(this))
    , m_speaker(new Speaker(this))
{
    // Coming up: nothing to hear yet, so the orb boots straight to rest.
    if (boot)
        m_orb->go(OrbState::State::StandBy);

    m_patience.setSingleShot(true);
    m_patience.setInterval(8000); // no speech for this long: let the mic go

    connect(m_mic, &Mic::speechStarted, this, &Controller::onSpeechStarted);
    connect(m_mic, &Mic::speechEnded, this, &Controller::onSpeechEnded);
    connect(m_mic, &Mic::availableChanged, this, &Controller::availableChanged);
    connect(m_mic, &Mic::availableChanged, this, [this] {
        if (!m_mic->available())
            m_orb->fail(tr("No microphone"));
    });

    connect(&m_patience, &QTimer::timeout, this, &Controller::onPatience);

    connect(m_speaker, &Speaker::levelChanged, this, [this] {
        m_level = m_speaker->level();
        emit levelChanged();
    });
    connect(m_speaker, &Speaker::finished, this, &Controller::onSpoke);
}

bool Controller::available() const
{
    return m_mic->isRunning();
}

QString Controller::voice() const
{
    return m_speaker->engine();
}

void Controller::talk()
{
    if (m_orb->state() == OrbState::State::Error)
        m_orb->go(OrbState::State::StandBy);

    if (!m_orb->go(OrbState::State::Listening))
        return;

    m_mic->start();
    if (!m_mic->isRunning()) {
        m_orb->fail(tr("No microphone"));
        return;
    }
    m_patience.start();
}

void Controller::say(const QString &text)
{
    m_orb->go(OrbState::State::Speaking);
    m_speaker->say(text);
}

void Controller::onSpeechStarted()
{
    m_patience.stop();
    m_orb->setBusy(true); // dotted: it can hear you
}

void Controller::onSpeechEnded()
{
    if (m_orb->state() != OrbState::State::Listening)
        return;

    m_orb->setBusy(false);
    m_orb->go(OrbState::State::Speaking);
    m_mic->stop();

    // The model is not installed yet. Until it is, the honest reply names that.
    m_speaker->say(tr("I heard you. The model is not installed yet."));
}

void Controller::onPatience()
{
    m_mic->stop();
    m_orb->go(OrbState::State::StandBy);
}

void Controller::onSpoke()
{
    m_level = 0.0;
    emit levelChanged();
    m_orb->go(OrbState::State::StandBy);
}
