#include "controller.h"

#include <QStandardPaths>

#include "audio/Mic.h"
#include "audio/Speaker.h"

namespace {
// Long enough to read the white arc as "warming up", short enough not to wait.
constexpr int BootMs = 1100;
} // namespace

Controller::Controller(OrbState *orb, bool boot, QObject *parent)
    : QObject(parent)
    , m_orb(orb)
    , m_mic(new Mic(this))
    , m_speaker(new Speaker(this))
{
    m_patience.setSingleShot(true);
    m_patience.setInterval(8000); // a sentence that never comes: let it rest

    connect(m_mic, &Mic::speechStarted, this, &Controller::onSpeechStarted);
    connect(m_mic, &Mic::speechEnded, this, &Controller::onSpeechEnded);
    connect(m_mic, &Mic::availableChanged, this, &Controller::availableChanged);
    connect(m_mic, &Mic::availableChanged, this, [this] {
        // A device that never opens is a real failure; one that closes because
        // we stopped it on purpose is not.
        if (!m_mic->available() && m_orb->state() == OrbState::State::Listening)
            m_orb->fail(tr("No microphone"));
    });

    connect(&m_patience, &QTimer::timeout, this, &Controller::onPatience);

    connect(m_speaker, &Speaker::levelChanged, this, [this] {
        m_level = m_speaker->level();
        emit levelChanged();
    });
    connect(m_speaker, &Speaker::finished, this, &Controller::onSpoke);

    QString why;
    m_wake = WakeModel::load(wakeModelPath(), &why);
    if (!m_wake.isReady())
        fprintf(stderr, "wake: no model yet (%s); any speech starts a turn\n", qPrintable(why));
    else
        fprintf(stderr, "wake: %d templates, threshold %.2f\n", m_wake.templates(), m_wake.threshold());

    if (boot) {
        // White arc while it comes up, then the turquoise rest state.
        QTimer::singleShot(BootMs, this, [this] { listen(); });
    }
}

bool Controller::available() const
{
    return m_mic->isRunning();
}

QString Controller::voice() const
{
    return m_speaker->engine();
}

// Always-on ears: the microphone stays open in StandBy, so speaking is what
// moves the orb, not a click.
void Controller::listen()
{
    if (m_orb->state() == OrbState::State::Booting)
        m_orb->go(OrbState::State::StandBy);

    m_mic->start();
    if (!m_mic->isRunning() && m_orb->state() == OrbState::State::StandBy)
        m_orb->fail(tr("No microphone"));
    m_patience.start();
}

void Controller::talk()
{
    if (m_orb->state() == OrbState::State::Error)
        m_orb->go(OrbState::State::StandBy);

    if (m_orb->state() == OrbState::State::StandBy)
        m_orb->go(OrbState::State::Listening);
    listen();
}

void Controller::say(const QString &text)
{
    m_orb->go(OrbState::State::Speaking);
    m_speaker->say(text);
}

void Controller::onSpeechStarted()
{
    m_patience.stop();

    // Speech in StandBy starts a turn on its own.
    if (m_orb->state() == OrbState::State::StandBy)
        m_orb->go(OrbState::State::Listening);

    m_orb->setBusy(true); // dotted: it can hear you
}

QString Controller::wakeModelPath()
{
    const QString dir = QStandardPaths::writableLocation(QStandardPaths::AppDataLocation);
    return dir + QStringLiteral("/wake.model");
}

void Controller::onSpeechEnded()
{
    if (m_orb->state() != OrbState::State::Listening)
        return;

    // Enrolled: only the wake word opens a turn. Not enrolled: any sentence
    // does, so the product is testable before anyone records their voice.
    if (m_wake.isReady()) {
        const double heard = m_wake.score(m_mic->utterance());
        fprintf(stderr, "wake: %.2f (threshold %.2f)\n", heard, m_wake.threshold());
        if (heard < m_wake.threshold()) {
            m_orb->setBusy(false);
            m_orb->go(OrbState::State::StandBy);
            m_mic->start();
            return;
        }
    }

    m_orb->setBusy(false);
    m_orb->go(OrbState::State::Speaking);

    // Barge-in is not a design decision, so the ears close while it talks.
    m_mic->stop();

    // No model yet: the honest reply says so. The chime still plays if the
    // machine has no local speech engine.
    m_speaker->say(tr("I heard you. The model is not installed yet."));
}

void Controller::onPatience()
{
    m_mic->start();
    if (m_orb->state() == OrbState::State::Listening)
        m_orb->go(OrbState::State::StandBy);
}

void Controller::onSpoke()
{
    m_level = 0.0;
    emit levelChanged();

    m_orb->go(OrbState::State::StandBy);
    m_patience.start();
    m_mic->start(); // ears back on for the next sentence
}
