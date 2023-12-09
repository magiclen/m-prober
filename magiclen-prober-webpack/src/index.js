/* eslint-disable @typescript-eslint/no-unsafe-return */
/* eslint-disable @typescript-eslint/no-floating-promises */
/* eslint-disable func-style */
/* eslint-disable @typescript-eslint/no-unsafe-argument */
/* eslint-disable @typescript-eslint/no-unsafe-member-access */
/* eslint-disable @typescript-eslint/no-unsafe-call */
/* eslint-disable @typescript-eslint/no-unsafe-assignment */

// eslint-disable-next-line import/order
import $ from "jquery";

import "bootstrap/js/dist/collapse.js";
import "bootstrap/js/dist/modal.js";
import Vue from "vue/dist/vue.esm.js";

import { applyRobotoMono } from "web-fonts/lib/roboto-mono.js";

import "./index.scss";

$.fn.pressEnter = function (fnc) {
    return this.each(function () {
        $(this).keypress(function (ev) {
            let keyCode = ev.keyCode ? ev.keyCode : ev.which;
            if (keyCode === 13) {
                fnc.call(this, ev);
                ev.preventDefault();
            }
        });
    });
};

function api(type, url, data, success, error, key) {
    if (type === "DELETE") {
        url = url + "?" + $.param(data);
        data = {};
    }

    let headers;
    if (typeof key === "string") {
        headers = { Authorization: key };
    }

    $.ajax({
        type: type,
        url: url,
        data: data,
        headers: headers,
        dataType: "json",
        success: function (response) {
            if (typeof success === "function") {
                success(response);
            }
        },
        error: function () {
            if (typeof success === "function") {
                error();
            }
        },
    });
}

function setElementDisplayNone(element, none = true) {
    if (!(element instanceof $)) {
        element = $(element);
    }
    if (none) {
        if (!element.hasClass("d-none")) {
            element.addClass("d-none");
        }
    } else if (element.hasClass("d-none")) {
        element.removeClass("d-none");
    }
}

function alertModal(body = "", title = undefined, close = "OK", afterClose = undefined, pressEnterToClose = true) {
    let m = $("#alertModal");
    let h = $(".modal-header", m);
    let t = $(".modal-title", h);
    let b = $(".modal-body", m);
    let cb = $(".modal-footer button", m);
    if (typeof title === "string") {
        t.text(title);
        setElementDisplayNone(h, false);
    } else {
        setElementDisplayNone(h, true);
    }
    b.html(body);
    cb.text(close);
    m.off("hidden.bs.modal");
    if (typeof afterClose === "function") {
        m.on("hidden.bs.modal", afterClose);
    }
    if (pressEnterToClose) {
        m.pressEnter(function () {
            m.modal("hide");
        });
    } else {
        m.pressEnter(function () {
            //
        });
    }
    m.modal("show");
}

function to(url) {
    location.href = url;
}

function go(url, e) {
    prevent(e);
    window.open(url);
}

function prevent(e) {
    if (!e) {
        e = window.event;
        if (!e) {
            return;
        }
    }
    e.cancelBubble = true;
    if (e.stopPropagation) {
        e.stopPropagation();
    }
}

function smoothScroll(el, offset = 0, duration = 500) {
    if (!(el instanceof $)) {
        el = $(el);
    }

    let body = $("html, body");

    let top = el.offset().top - parseFloat(body.css("margin-top")) - parseFloat(body.css("padding-top")) - parseFloat(el.css("margin-top")) - parseFloat(el.css("padding-top")) - offset;
    body.animate({ scrollTop: top }, duration, "swing");
}

function removeCardsBorderDark(el) {
    $("> .card", el).removeClass("border-dark");
}

function addBorderDark(el) {
    if (!el.hasClass("border-dark")) {
        el.addClass("border-dark");
    }
}

function toTag(el) {
    let wrapper = $("#wrapper");

    let paddingTop = parseFloat(wrapper.css("padding-top"));

    let marginTop = parseFloat(wrapper.css("margin-top"));

    smoothScroll(el, paddingTop + marginTop);
}

function callMonitorAPI(vueData, authKey = undefined, interval, retryCount = 0) {
    let t = new Date().getTime();

    api("GET", "api/monitor", {}, function (data) {
        let code = data.code;

        if (code !== 0) {
            return;
        }

        data = data.data;

        vueData.hostname = data.hostname;
        vueData.kernel = data.kernel;
        vueData.rtc_time = data.rtc_time;
        vueData.uptime = data.uptime;

        vueData.logical_cores = data.cpus.map(function (cpu) {
            return cpu.threads;
        }).reduce(function (sum, n) {
            return sum + n;
        });

        vueData.load_average = data.load_average;

        for (let field in vueData.load_average_scale) {
            if (Object.prototype.hasOwnProperty.call(vueData.load_average_scale, field)) {
                vueData.load_average_scale[field] = data.load_average[field] * 100 / vueData.logical_cores;
            }
        }

        vueData.cpu = data.cpus_stat[0] * 100;

        let cpuThreadOffset = 1;

        data.cpus.forEach(function (cpu) {
            let threads = cpu.threads;

            cpu.cpus_stat = [];

            let e = cpuThreadOffset + threads;

            if (e > data.length) {
                e = data.length;
            }

            for (let i = cpuThreadOffset;i < e;++i) {
                cpu.cpus_stat.push(data.cpus_stat[i] * 100);
            }

            cpuThreadOffset += threads;
        });

        vueData.cpus = data.cpus;

        vueData.memory = data.memory;
        vueData.swap = data.swap;

        for (let field in vueData.memory_scale) {
            if (Object.prototype.hasOwnProperty.call(vueData.memory_scale, field)) {
                vueData.memory_scale[field] = data.memory[field].value * 100 / data.memory.total.value;
            }
        }

        for (let field in vueData.swap_scale) {
            if (Object.prototype.hasOwnProperty.call(vueData.swap_scale, field)) {
                vueData.swap_scale[field] = data.swap[field].value * 100 / data.swap.total.value;
            }
        }

        vueData.network = data.network;

        data.volumes.forEach(function (volume) {
            volume.scale = volume.used.value * 100 / volume.size.value;
        });

        vueData.volumes = data.volumes;

        let now = new Date();

        vueData.last_update_time = now.toLocaleString();

        let nt = now.getTime();

        let d = nt - t;

        let timeout = 0;

        if (d < interval) {
            timeout = interval - d;
        }

        setTimeout(function () {
            callMonitorAPI(vueData, authKey, interval, 0);
        }, timeout);
    }, function () {
        if (retryCount >= 10) {
            alertModal("The monitor API can not be invoked successfully. Please refresh this page to try again.", "Error", "OK", function () {
                to("");
            }, false);
        } else {
            console.warn("Retry to call the monitor API in 1 second.");

            setTimeout(function () {
                callMonitorAPI(vueData, authKey, interval, retryCount + 1);
            }, 1000);
        }
    }, authKey);
}

export function monitorInit() {
    applyRobotoMono("html body, code");

    $("#menu-github").click(function (e) {
        e.preventDefault();
        go("https://github.com/magiclen/m-prober");
    });

    let monitorData = {
        last_update_time: "Never",
        kernel: "",
        hostname: "",
        rtc_time: "",
        uptime: {
            value: 0,
            text: "",
        },
        logical_cores: 0,
        load_average: {
            one: 0,
            five: 0,
            fifteen: 0,
        },
        load_average_scale: {
            one: 0,
            five: 0,
            fifteen: 0,
        },
        cpu: 0,
        cpus: [],
        memory: {
            total: {
                value: 0,
                text: "0 B",
            },
            used: {
                value: 0,
                text: "0 B",
            },
            buffer_cache: {
                value: 0,
                text: "0 B",
            },
        },
        memory_scale: {
            used: 0,
            buffer_cache: 0,
        },
        swap: {
            total: {
                value: 0,
                text: "0 B",
            },
            used: {
                value: 0,
                text: "0 B",
            },
            cache: {
                value: 0,
                text: "0 B",
            },
        },
        swap_scale: {
            used: 0,
            cache: 0,
        },
        network: [],
        volumes: [],
    };

    new Vue({
        el: "#monitor",
        data: monitorData,
        mounted: function () {
            let monitorEL = this.$el;

            $("#menu-toggle").click(function (e) {
                e.preventDefault();
                $("#wrapper").toggleClass("toggled");
                removeCardsBorderDark(monitorEL);
            });

            new Vue({
                el: "#sidebar-wrapper",
                data: {},
                mounted: function () {
                    let authKey = $("#auth-key");

                    if (authKey.length > 0) {
                        authKey = authKey.val();
                    } else {
                        authKey = undefined;
                    }

                    let timeInterval = parseInt($("#time-interval").val());

                    callMonitorAPI(monitorData, authKey, timeInterval);
                },
                methods: {
                    toLinuxInformation: function () {
                        removeCardsBorderDark(monitorEL);

                        let el = $("> #linux-information", monitorEL);

                        toTag(el);
                        addBorderDark(el);
                    },
                    toLoadAverage: function () {
                        removeCardsBorderDark(monitorEL);

                        let el = $("> #load-average", monitorEL);

                        toTag(el);
                        addBorderDark(el);
                    },
                    toCPUs: function () {
                        removeCardsBorderDark(monitorEL);

                        let el = $("> #cpus", monitorEL);

                        toTag(el);
                        addBorderDark(el);
                    },
                    toMemory: function () {
                        removeCardsBorderDark(monitorEL);

                        let el = $("> #memory", monitorEL);

                        toTag(el);
                        addBorderDark(el);
                    },
                    toNetworks: function () {
                        removeCardsBorderDark(monitorEL);

                        let el = $("> #networks", monitorEL);

                        toTag(el);
                        addBorderDark(el);
                    },
                    toVolumes: function () {
                        removeCardsBorderDark(monitorEL);

                        let el = $("> #volumes", monitorEL);

                        toTag(el);
                        addBorderDark(el);
                    },
                },
            });
        },
    });
}
