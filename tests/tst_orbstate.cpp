#include <QSignalSpy>
#include <QtTest>

#include "orbstate.h"

namespace {
using State = OrbState::State;

// Walk to a state along legal edges only, so a failure here means the machine
// itself is wrong, not the test.
void reach(OrbState &orb, State target)
{
    switch (target) {
    case State::Booting:
        break;
    case State::StandBy:
        orb.go(State::StandBy);
        break;
    case State::Listening:
        orb.go(State::StandBy);
        orb.go(State::Listening);
        break;
    case State::Speaking:
        orb.go(State::StandBy);
        orb.go(State::Listening);
        orb.go(State::Speaking);
        break;
    case State::Error:
        orb.fail(QStringLiteral("setup"));
        break;
    }
}
} // namespace

class TestOrbState : public QObject
{
    Q_OBJECT

private slots:
    void initialState()
    {
        OrbState orb;
        QCOMPARE(orb.stateName(), QStringLiteral("booting"));
        QVERIFY(!orb.busy());
        QVERIFY(orb.error().isEmpty());
    }

    void bootToStandBy()
    {
        OrbState orb;
        QVERIFY(orb.go(State::StandBy));
        QCOMPARE(orb.stateName(), QStringLiteral("standby"));
    }

    void selfTransitionRejected()
    {
        OrbState orb;
        orb.go(State::StandBy);
        QSignalSpy spy(&orb, &OrbState::stateChanged);

        QVERIFY(!orb.go(State::StandBy));
        QCOMPARE(orb.stateName(), QStringLiteral("standby"));
        QCOMPARE(spy.count(), 0);
    }

    void bootCannotJumpToSpeaking()
    {
        OrbState orb;
        QVERIFY(!orb.go(State::Speaking));
        QVERIFY(!orb.go(State::Listening));
        QCOMPARE(orb.stateName(), QStringLiteral("booting"));
    }

    void busyOnlyWhileListening()
    {
        OrbState orb;
        orb.setBusy(true);
        QVERIFY2(!orb.busy(), "busy must be ignored in booting");

        reach(orb, State::Listening);
        orb.setBusy(true);
        QVERIFY(orb.busy());
    }

    void firstAudioToSpeaking()
    {
        OrbState orb;
        reach(orb, State::Listening);
        orb.setBusy(true);

        QVERIFY(orb.go(State::Speaking));
        QVERIFY2(!orb.busy(), "leaving listening clears the working flag");
    }

    void bargeInReturnsToListening()
    {
        OrbState orb;
        reach(orb, State::Speaking);
        QVERIFY(orb.go(State::Listening));
        QCOMPARE(orb.stateName(), QStringLiteral("listening"));
    }

    void failFromEveryState_data()
    {
        QTest::addColumn<int>("from");
        QTest::newRow("booting") << int(State::Booting);
        QTest::newRow("standby") << int(State::StandBy);
        QTest::newRow("listening") << int(State::Listening);
        QTest::newRow("speaking") << int(State::Speaking);
        QTest::newRow("error") << int(State::Error);
    }

    void failFromEveryState()
    {
        QFETCH(int, from);
        OrbState orb;
        reach(orb, State(from));

        orb.fail(QStringLiteral("microphone lost"));

        QCOMPARE(orb.stateName(), QStringLiteral("error"));
        QCOMPARE(orb.error(), QStringLiteral("microphone lost"));
    }

    void errorCannotSkipToSpeaking()
    {
        OrbState orb;
        reach(orb, State::Error);
        QVERIFY(!orb.go(State::Speaking));
        QVERIFY(!orb.go(State::Listening));
        QCOMPARE(orb.stateName(), QStringLiteral("error"));
    }

    void recoveryClearsTheError()
    {
        OrbState orb;
        reach(orb, State::Error);
        QVERIFY(orb.go(State::StandBy));
        QVERIFY2(orb.error().isEmpty(), "a recovered orb must not keep a stale reason");
    }

    // The QML side switches on these exact strings. A rename without a QML
    // change would show a blank ring, so the strings are part of the contract.
    void stateNamesMatchQml()
    {
        QCOMPARE(OrbState::name(State::Booting), QStringLiteral("booting"));
        QCOMPARE(OrbState::name(State::StandBy), QStringLiteral("standby"));
        QCOMPARE(OrbState::name(State::Listening), QStringLiteral("listening"));
        QCOMPARE(OrbState::name(State::Speaking), QStringLiteral("speaking"));
        QCOMPARE(OrbState::name(State::Error), QStringLiteral("error"));
    }

    void advanceWalksTheCycle()
    {
        OrbState orb;
        QVERIFY(orb.advance());
        QCOMPARE(orb.stateName(), QStringLiteral("standby"));
        QVERIFY(orb.advance());
        QCOMPARE(orb.stateName(), QStringLiteral("listening"));
        QVERIFY(orb.advance());
        QVERIFY(orb.busy());
        QVERIFY(orb.advance());
        QCOMPARE(orb.stateName(), QStringLiteral("speaking"));
        QVERIFY(orb.advance());
        QCOMPARE(orb.stateName(), QStringLiteral("error"));
        QVERIFY(orb.advance());
        QCOMPARE(orb.stateName(), QStringLiteral("standby"));
    }

    // The canvas draws Speaking -> StandBy. Leaving speaking directly to rest is
    // the normal end of a turn and must not need a detour through listening.
    void speakingEndsAtStandBy()
    {
        OrbState orb;
        reach(orb, State::Speaking);
        QVERIFY(orb.go(State::StandBy));
        QCOMPARE(orb.stateName(), QStringLiteral("standby"));
    }

    void busyChangesEmitOncePerChange()
    {
        OrbState orb;
        reach(orb, State::Listening);
        QSignalSpy spy(&orb, &OrbState::busyChanged);

        orb.setBusy(true);
        orb.setBusy(true);
        orb.setBusy(false);
        orb.setBusy(false);

        QCOMPARE(spy.count(), 2);
    }

    void rejectedMoveIsSilent()
    {
        OrbState orb; // booting
        QSignalSpy stateSpy(&orb, &OrbState::stateChanged);
        QSignalSpy busySpy(&orb, &OrbState::busyChanged);

        orb.setBusy(true); // ignored outside listening
        QVERIFY(!orb.go(State::Speaking));

        QCOMPARE(stateSpy.count(), 0);
        QCOMPARE(busySpy.count(), 0);
    }

    void eachAcceptedMoveEmitsOneSignal()
    {
        OrbState orb;
        QSignalSpy spy(&orb, &OrbState::stateChanged);

        orb.go(State::StandBy);
        orb.go(State::Listening);
        orb.go(State::Speaking);
        orb.go(State::StandBy);

        QCOMPARE(spy.count(), 4);
    }

    void failingTwiceKeepsTheLatestReason()
    {
        OrbState orb;
        reach(orb, State::Error);

        orb.fail(QStringLiteral("google unreachable"));

        QCOMPARE(orb.stateName(), QStringLiteral("error"));
        QCOMPARE(orb.error(), QStringLiteral("google unreachable"));
    }

    void busyIsFalseAfterError()
    {
        OrbState orb;
        reach(orb, State::Listening);
        orb.setBusy(true);
        QVERIFY(orb.busy());

        orb.fail(QStringLiteral("timeout"));

        QVERIFY2(!orb.busy(), "a failed turn must not stay in the working phase");
    }
};

QTEST_GUILESS_MAIN(TestOrbState)
#include "tst_orbstate.moc"
