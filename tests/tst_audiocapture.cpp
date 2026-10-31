#include <QDir>
#include <QSignalSpy>
#include <QTemporaryDir>
#include <QtTest>

#include "audio/AudioCapture.h"

namespace {

// A capture with no microphone: both device seams are faked, so the ring and
// the signal are tested for real without opening any hardware.
class FakeCapture : public AudioCapture
{
public:
    using AudioCapture::AudioCapture;

protected:
    bool openSource() override { return true; }
    void closeSource() override {}
};

// A silent frame of the given sample count, 16-bit mono.
QByteArray frame(int samples)
{
    return QByteArray(samples * AudioCapture::BytesPerSample, '\0');
}

} // namespace

class TestAudioCapture : public QObject
{
    Q_OBJECT

private slots:
    void frameIsSixteenBitMono()
    {
        QCOMPARE(AudioCapture::SampleRate, 16000);
        QCOMPARE(AudioCapture::Channels, 1);

        FakeCapture capture;
        QSignalSpy spy(&capture, &AudioCapture::frameReady);
        capture.start();

        capture.pushFrameForTest(frame(160));

        QCOMPARE(spy.count(), 1);
        QCOMPARE(spy.at(0).at(0).toByteArray().size(), 320);
        QCOMPARE(capture.bufferedSamples(), 160);
        QCOMPARE(capture.bufferedBytes(), 320);
    }

    void ringHoldsAtMostThreeSeconds()
    {
        FakeCapture capture;
        capture.start();

        // Ten seconds of audio in 20 ms frames: 500 * 320 samples.
        for (int i = 0; i < 500; ++i)
            capture.pushFrameForTest(frame(320));

        QCOMPARE(capture.bufferedSamples(), AudioCapture::MaxBufferedSamples);
        QCOMPARE(capture.bufferedBytes(), AudioCapture::MaxBufferedBytes);
        QVERIFY2(capture.bufferedBytes() <= 3 * AudioCapture::SampleRate * 2,
                 "the ring grew past three seconds");
    }

    // One oversized chunk must be trimmed too, not appended whole.
    void oversizedChunkIsTrimmed()
    {
        FakeCapture capture;
        capture.start();

        capture.pushFrameForTest(frame(5 * AudioCapture::SampleRate));

        QCOMPARE(capture.bufferedSamples(), AudioCapture::MaxBufferedSamples);
    }

    void startAndStopAreIdempotent()
    {
        FakeCapture capture;
        QVERIFY(!capture.isRunning());

        capture.start();
        QVERIFY(capture.isRunning());
        capture.start();
        QVERIFY2(capture.isRunning(), "a second start must change nothing");

        capture.stop();
        QVERIFY(!capture.isRunning());
        capture.stop();
        QVERIFY2(!capture.isRunning(), "a second stop must change nothing");
    }

    // A device that goes away mid-capture is silent and can be reopened.
    void lostDeviceIsSilentAndRestartable()
    {
        FakeCapture capture;
        QSignalSpy spy(&capture, &AudioCapture::frameReady);
        capture.start();

        capture.pushFrameForTest(frame(160));
        QCOMPARE(spy.count(), 1);

        capture.stop(); // the device vanished
        capture.pushFrameForTest(frame(160));
        QCOMPARE(spy.count(), 1);

        capture.start();
        QVERIFY(capture.isRunning());
        capture.pushFrameForTest(frame(160));
        QCOMPARE(spy.count(), 2);
    }

    void stoppedCaptureIgnoresFrames()
    {
        FakeCapture capture;
        QSignalSpy spy(&capture, &AudioCapture::frameReady);

        capture.pushFrameForTest(frame(160));
        QCOMPARE(spy.count(), 0);
        QCOMPARE(capture.bufferedSamples(), 0);
    }

    // The app data path and the temp path point at one scratch directory. If
    // capture wrote anything at all, it would show up here.
    void nothingIsWrittenToDisk()
    {
        QTemporaryDir scratch;
        QVERIFY(scratch.isValid());
        qputenv("XDG_DATA_HOME", scratch.path().toLocal8Bit());
        qputenv("TMPDIR", scratch.path().toLocal8Bit());

        FakeCapture capture;
        capture.start();
        for (int i = 0; i < 100; ++i)
            capture.pushFrameForTest(frame(320));
        capture.stop();

        QVERIFY2(QDir(scratch.path()).isEmpty(),
                 qPrintable(QStringLiteral("capture left files behind: %1")
                                .arg(QDir(scratch.path()).entryList(QDir::AllEntries | QDir::Hidden | QDir::System)
                                         .join(QLatin1Char(',')))));
    }
};

QTEST_GUILESS_MAIN(TestAudioCapture)
#include "tst_audiocapture.moc"
