#include <QtTest>

#include <cmath>

#include "audio/WakeModel.h"

namespace {

constexpr int Rate = 16000;

// A synthetic spoken word: a glide whose pitch moves, with a little noise so
// two takes are not bit-identical. Real recordings replace this at enrolment;
// it exists to prove the matcher separates one shape from another.
QVector<qint16> word(int ms, double fromHz, double toHz, double shiftMs = 0, int seed = 0)
{
    const int n = Rate * ms / 1000;
    QVector<qint16> out(n);
    unsigned rng = 12345u + seed;
    for (int i = 0; i < n; ++i) {
        const double t = double(i) / Rate;
        const double frac = double(i) / n;
        const double hz = fromHz + (toHz - fromHz) * frac;
        const double env = std::sin(M_PI * frac); // fade in and out
        rng = rng * 1664525u + 1013904223u;
        const double noise = (double(rng % 1000) / 1000.0 - 0.5) * 0.02;
        const double shift = shiftMs > 0 ? std::sin(2 * M_PI * (fromHz * 0.5) * t) * 0.0 : 0.0;
        out[i] = qint16((std::sin(2 * M_PI * hz * t) * 0.35 + noise) * env * 32767 + shift);
    }
    return out;
}

QVector<qint16> other()
{
    const int n = Rate * 600 / 1000;
    QVector<qint16> out(n);
    for (int i = 0; i < n; ++i) {
        const double t = double(i) / Rate;
        const double hz = (i / 1600) % 2 == 0 ? 500.0 : 1500.0;
        out[i] = qint16(std::sin(2 * M_PI * hz * t) * 0.3 * 32767);
    }
    return out;
}

} // namespace

class WakeModelTest : public QObject
{
    Q_OBJECT

private slots:
    void an_utterance_matches_itself()
    {
        WakeModel model;
        const auto take = word(600, 300, 2200);
        QVERIFY(model.enroll(take));
        QVERIFY2(model.score(take) > 0.8, qPrintable(QString::number(model.score(take))));
    }

    void a_second_take_of_the_same_word_matches()
    {
        WakeModel model;
        QVERIFY(model.enroll(word(600, 300, 2200)));
        const double again = model.score(word(560, 320, 2100, 0, 7));
        QVERIFY2(again > 0.50, qPrintable(QString::number(again)));
    }

    void a_different_sound_is_refused()
    {
        WakeModel model;
        QVERIFY(model.enroll(word(600, 300, 2200)));
        const double no = model.score(other());
        QVERIFY2(no < 0.40, qPrintable(QString::number(no)));
    }

    void calibration_tightens_a_consistent_voice()
    {
        WakeModel model;
        QVERIFY(model.enroll(word(600, 300, 2200)));
        QVERIFY(model.enroll(word(600, 300, 2200)));
        const double before = model.threshold();
        const double after = model.calibrate();
        QVERIFY2(after > before, qPrintable(QString::number(after)));
        QVERIFY(after <= 0.90);
        // A different sound now has to clear the stricter gate.
        QVERIFY(model.score(other()) < after);
    }

    void a_clip_too_short_to_be_a_word_is_not_enrolled()
    {
        WakeModel model;
        QVERIFY(!model.enroll(word(50, 300, 2200)));
        QCOMPARE(model.templates(), 0);
        QVERIFY(!model.isReady());
    }

    void an_empty_model_scores_nothing()
    {
        const WakeModel model;
        QCOMPARE(model.score(word(600, 300, 2200)), 0.0);
    }

    void the_model_survives_a_disk_round_trip()
    {
        QTemporaryDir dir;
        const QString path = dir.filePath("wake.model");

        WakeModel model;
        QVERIFY(model.enroll(word(600, 300, 2200)));
        QVERIFY(model.save(path));

        QString error;
        const WakeModel loaded = WakeModel::load(path, &error);
        QVERIFY2(loaded.isReady(), qPrintable(error));
        QCOMPARE(loaded.templates(), 1);
        QVERIFY(qAbs(loaded.score(word(600, 300, 2200)) - model.score(word(600, 300, 2200))) < 1e-4);
    }

    void a_corrupt_file_is_refused_not_trusted()
    {
        QTemporaryDir dir;
        const QString path = dir.filePath("wake.model");
        {
            QFile f(path);
            QVERIFY(f.open(QIODevice::WriteOnly));
            f.write("not a model\n");
        }
        QString error;
        const WakeModel loaded = WakeModel::load(path, &error);
        QVERIFY(!loaded.isReady());
        QVERIFY(!error.isEmpty());
    }
};

QTEST_GUILESS_MAIN(WakeModelTest)
#include "tst_wakemodel.moc"
