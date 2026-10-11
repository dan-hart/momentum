// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum

import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.graphics.Path
import androidx.graphics.path.PathIterator
import androidx.graphics.path.PathSegment
import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import kotlin.math.hypot

/** Runs on API26's native iterator and on API34+'s native conic conversion. */
@RunWith(AndroidJUnit4::class)
class NativeGraphicsPathTest {
    @Test fun nativeIteratorKeepsVerbOrderAndControlPoints() {
        val path = Path().apply {
            moveTo(3f, 5f)
            lineTo(7f, 11f)
            quadTo(13f, 17f, 19f, 23f)
            cubicTo(29f, 31f, 37f, 41f, 43f, 47f)
            close()
        }
        val iterator = PathIterator(path, PathIterator.ConicEvaluation.AsConic)
        assertEquals(5, iterator.calculateSize())
        val segments = iterator.asSequence().toList()
        assertEquals(listOf(PathSegment.Type.Move, PathSegment.Type.Line, PathSegment.Type.Quadratic,
            PathSegment.Type.Cubic, PathSegment.Type.Close), segments.map { it.type })
        assertEquals(3f, segments[0].points[0].x, 0f)
        assertEquals(11f, segments[1].points[1].y, 0f)
        assertEquals(13f, segments[2].points[1].x, 0f)
        assertEquals(23f, segments[2].points[2].y, 0f)
        assertEquals(37f, segments[3].points[2].x, 0f)
        assertEquals(47f, segments[3].points[3].y, 0f)
        assertFalse(iterator.hasNext())
    }

    @Test fun nativeConicConversionRendersTheSameClosedOval() {
        val original = Path().apply { addOval(8f, 8f, 56f, 56f, Path.Direction.CW) }
        val raw = PathIterator(original, PathIterator.ConicEvaluation.AsConic).asSequence().toList()
        assertEquals(4, raw.count { it.type == PathSegment.Type.Conic })
        assertTrue(raw.filter { it.type == PathSegment.Type.Conic }.all { it.weight in 0.70f..0.71f })
        // The pinned upstream calculateSize(true) leaves converted conics in its
        // instance's converter on both API boundaries. Count on a separate
        // iterator so this JNI/render gate exercises an untouched conversion.
        val expectedSize = PathIterator(original, PathIterator.ConicEvaluation.AsQuadratics, 0.01f)
            .calculateSize(includeConvertedConics = true)
        val segments = PathIterator(original, PathIterator.ConicEvaluation.AsQuadratics, 0.01f)
            .asSequence().toList()
        assertEquals(expectedSize, segments.size)
        assertEquals(PathSegment.Type.Move, segments.first().type)
        assertEquals(PathSegment.Type.Close, segments.last().type)
        assertFalse(segments.any { it.type == PathSegment.Type.Conic })
        assertTrue(segments.count { it.type == PathSegment.Type.Quadratic } >= 4)
        var endpoint = segments.first().points.single()
        segments.filter { it.type == PathSegment.Type.Quadratic }.forEach { segment ->
            assertEquals(endpoint.x, segment.points[0].x, 0f)
            assertEquals(endpoint.y, segment.points[0].y, 0f)
            // Check actual converted geometry independently of Skia's edge rasterization.
            for (sample in 0..100) {
                val t = sample / 100.0
                val u = 1.0 - t
                val points = segment.points
                val x = u * u * points[0].x + 2 * u * t * points[1].x + t * t * points[2].x
                val y = u * u * points[0].y + 2 * u * t * points[1].y + t * t * points[2].y
                assertEquals("Converted conic stays on the original circle", 24.0,
                    hypot(x - 32, y - 32), 0.01)
            }
            endpoint = segment.points[2]
        }
        assertEquals(segments.first().points.single().x, endpoint.x, 0.001f)
        assertEquals(segments.first().points.single().y, endpoint.y, 0.001f)
        val rebuilt = Path()
        segments.forEach { segment ->
            val points = segment.points
            assertTrue(points.all { it.x.isFinite() && it.y.isFinite() })
            when (segment.type) {
                PathSegment.Type.Move -> rebuilt.moveTo(points[0].x, points[0].y)
                PathSegment.Type.Line -> rebuilt.lineTo(points[1].x, points[1].y)
                PathSegment.Type.Quadratic -> rebuilt.quadTo(points[1].x, points[1].y, points[2].x, points[2].y)
                PathSegment.Type.Cubic -> rebuilt.cubicTo(points[1].x, points[1].y, points[2].x, points[2].y,
                    points[3].x, points[3].y)
                PathSegment.Type.Close -> rebuilt.close()
                PathSegment.Type.Done -> Unit
                PathSegment.Type.Conic -> error("Conic must have been converted")
            }
        }
        val reference = render(original)
        val actual = render(rebuilt)
        try {
            assertEquals(Color.RED, actual.getPixel(32, 32))
            assertEquals(Color.TRANSPARENT, actual.getPixel(0, 0))
            var differences = 0
            for (y in 0 until 64) for (x in 0 until 64) {
                if (reference.getPixel(x, y) != actual.getPixel(x, y)) differences++
            }
            assertTrue("Converted native path must preserve rendered coverage; changed $differences pixels",
                differences <= 64)
        } finally { reference.recycle(); actual.recycle() }
    }

    private fun render(path: Path): Bitmap = Bitmap.createBitmap(64, 64, Bitmap.Config.ARGB_8888).also {
        // Paint() enables antialiasing by default from API31. This gate compares
        // binary fill coverage on every API, rather than exact edge alpha values.
        Canvas(it).drawPath(path, Paint().apply {
            color = Color.RED
            style = Paint.Style.FILL
            isAntiAlias = false
        })
    }
}
