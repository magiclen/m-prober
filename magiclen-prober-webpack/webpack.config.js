const MiniCssExtractPlugin = require('mini-css-extract-plugin');

const TerserPlugin = require('terser-webpack-plugin');

const path = require('path');
const glob = require('glob');
const PurgecssPlugin = require('purgecss-webpack-plugin');

const PATHS = {
    views: path.resolve(__dirname, '..', 'views'),
};

function collectWhitelist() {
    return ['toggled', 'collapsed', 'collapsing', 'show', 'modal-backdrop', 'modal-open', 'd-none', 'border-dark'];
}

module.exports = {
    entry: {
        'bundle': './src/app.js',
        'font-roboto-mono': './src/font-roboto-mono.scss',
    },
    output: {
        filename: './js/[name].min.js',
        libraryTarget: "umd",
        clean: true,
    },
    plugins: [
        new MiniCssExtractPlugin({
            filename: './css/[name].min.css',
        }),
        new PurgecssPlugin({
            paths: glob.sync(`${PATHS.views}/**/*`, {nodir: true}),
            safelist() {
                return {
                    standard: collectWhitelist(),
                    deep: [],
                    greedy: []
                }
            }
        })
    ],
    module: {
        rules: [
            {
                test: /\.js$/,
                use: {
                    loader: 'babel-loader',
                    options: {
                        presets: ['@babel/preset-env']
                    }
                }
            },
            {
                test: /\.(sa|sc|c)ss$/,
                use: [
                    { loader: MiniCssExtractPlugin.loader },
                    "css-loader",
                    {
                        loader: "postcss-loader",
                        options: {
                            postcssOptions: {
                                plugins: [
                                    require("autoprefixer"),
                                    require("cssnano")({ preset: ["default", { discardComments: { removeAll: true } }] }),
                                ],
                            },
                        },
                    },
                    "sass-loader",
                ],
            },
            {
                test: /\.(eot|woff|woff2|[ot]tf)$/,
                type: 'asset/resource',
                generator: {
                    filename: 'fonts/[name][ext]',
                }
            },
            {
                test: /.*font.*\.svg$/,
                type: 'asset/resource',
                generator: {
                    filename: 'fonts/[name][ext]',
                }
            }
        ]
    },
    optimization: { minimizer: [new TerserPlugin({ extractComments: false, terserOptions: { format: { comments: false } } })] },
    resolve: {
        alias: {
            'vue$': 'vue/dist/vue.esm.js'
        }
    }
};