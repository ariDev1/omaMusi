//! GTK stage that selects a software surface when hardware rendering fails.
use super::gl::GlRenderer;
use anyhow::{Context, Result};
use gtk::prelude::*;
use std::{cell::RefCell, rc::Rc, time::Instant};

pub struct RenderCanvas {
    pub widget: gtk::Stack,
    pub gl_area: gtk::GLArea,
    pub cpu_area: gtk::DrawingArea,
    renderer: Rc<RefCell<Option<GlRenderer>>>,
    failure: Rc<RefCell<Option<String>>>,
}
impl RenderCanvas {
    pub fn new(
        draw_cpu: impl Fn(&cairo::Context, i32, i32) -> Result<()> + 'static,
        draw_gpu: impl Fn(&mut GlRenderer, i32, i32, f64, f64) -> Result<()> + 'static,
    ) -> Self {
        let widget = gtk::Stack::new();
        widget.set_hexpand(true);
        widget.set_vexpand(true);
        let gl_area = gtk::GLArea::new();
        gl_area.set_required_version(3, 3);
        gl_area.set_use_es(false);
        gl_area.set_has_depth_buffer(false);
        gl_area.set_hexpand(true);
        gl_area.set_vexpand(true);
        gl_area.set_can_target(false);
        let cpu_area = gtk::DrawingArea::new();
        cpu_area.set_hexpand(true);
        cpu_area.set_vexpand(true);
        cpu_area.set_can_target(false);
        widget.add_named(&gl_area, Some("gpu"));
        widget.add_named(&cpu_area, Some("cpu"));
        widget.set_visible_child_name("gpu");
        let renderer = Rc::new(RefCell::new(None));
        let failure = Rc::new(RefCell::new(None));
        cpu_area.set_draw_func(move |_, ctx, width, height| {
            if let Err(error) = draw_cpu(ctx, width, height) {
                eprintln!("omaMusi software rendering: {error:#}");
            }
        });
        {
            let renderer = renderer.clone();
            let failure = failure.clone();
            let stack = widget.downgrade();
            gl_area.connect_realize(move |area| {
                area.make_current();
                let result = (|| -> Result<GlRenderer> {
                    if let Some(error) = area.error() {
                        return Err(error.into());
                    }
                    let context = area.context().context("No OpenGL context")?;
                    GlRenderer::new(&context)
                })();
                match result {
                    Ok(backend) => {
                        eprintln!("omaMusi: renderer=gpu ({})", backend.description());
                        renderer.replace(Some(backend));
                    }
                    Err(error) => {
                        let detail = format!("OpenGL initialization: {error:#}");
                        eprintln!("omaMusi: renderer=cpu ({detail})");
                        failure.replace(Some(detail));
                        if let Some(stack) = stack.upgrade() {
                            stack.set_visible_child_name("cpu");
                        }
                    }
                }
            });
        }
        {
            let renderer = renderer.clone();
            let failure = failure.clone();
            let stack = widget.downgrade();
            let clock = RefCell::new(Instant::now());
            gl_area.connect_render(move |area, _| {
                let now = Instant::now();
                let dt = now.duration_since(*clock.borrow()).as_secs_f64();
                clock.replace(now);
                let result = {
                    let mut borrowed = renderer.borrow_mut();
                    if let Some(backend) = borrowed.as_mut() {
                        let ratio = area.scale_factor() as f64;
                        draw_gpu(
                            backend,
                            area.width() * area.scale_factor(),
                            area.height() * area.scale_factor(),
                            ratio,
                            dt,
                        )
                    } else {
                        Ok(())
                    }
                };
                if let Err(error) = result {
                    let detail = format!("OpenGL rendering: {error:#}");
                    eprintln!("omaMusi: renderer=cpu ({detail})");
                    failure.replace(Some(detail));
                    if let Some(stack) = stack.upgrade() {
                        stack.set_visible_child_name("cpu");
                    }
                }
                gtk::glib::Propagation::Stop
            });
        }
        {
            let renderer = renderer.clone();
            gl_area.connect_unrealize(move |_| {
                if let Some(mut backend) = renderer.borrow_mut().take() {
                    backend.context().make_current();
                    let _ = backend.cleanup();
                }
            });
        }
        Self {
            widget,
            gl_area,
            cpu_area,
            renderer,
            failure,
        }
    }
    pub fn queue_draw(&self) {
        if self.is_cpu() {
            self.cpu_area.queue_draw();
        } else {
            self.gl_area.queue_render();
        }
    }
    pub fn is_cpu(&self) -> bool {
        self.widget.visible_child_name().as_deref() == Some("cpu")
    }
    pub fn failure(&self) -> Option<String> {
        self.failure.borrow().clone()
    }
    pub fn description(&self) -> String {
        if self.is_cpu() {
            format!("cpu ({})", self.failure().unwrap_or_default())
        } else {
            self.renderer
                .borrow()
                .as_ref()
                .map(|r| format!("gpu ({})", r.description()))
                .unwrap_or_else(|| "initializing".into())
        }
    }
}
