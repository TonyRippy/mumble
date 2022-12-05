const Path = require('path');
const Webpack = require('webpack');
const CopyWebpackPlugin = require('copy-webpack-plugin');
const MiniCssExtractPlugin = require('mini-css-extract-plugin');

module.exports = {
  entry: {
    'js/index': './src/index.ts',
    'css/index': './src/index.scss',
  },
  devtool: 'inline-source-map',
  mode: 'development',
  output: {
    path: Path.resolve(__dirname, 'dist'),
    filename: '[name].min.js',
  },
  plugins: [
    new Webpack.ProvidePlugin({
      'mdb': 'mdb',
    }),
    new CopyWebpackPlugin({
      patterns: [
        { from: Path.resolve(__dirname, './src/index.html') },
        { from: Path.resolve(__dirname, './src/img'), to: 'img' },
      ],
    }),
    new MiniCssExtractPlugin({
      filename: '[name].min.css',
    }),
  ],
  resolve: {
    alias: {
      '~': Path.resolve(__dirname, './src'),
      'mdb': Path.join(__dirname, './node_modules/mdb-ui-kit'),
    },
  },
  module: {
    rules: [
      {
        test: /\.tsx?$/,
        use: 'ts-loader',
        exclude: /node_modules/,
      },
      {
        test: /\.s?css/i,
        use: [MiniCssExtractPlugin.loader, 'css-loader', 'postcss-loader', 'sass-loader'],
      },
    ],
  },
  resolve: {
    extensions: ['.tsx', '.ts', '.js'],
  },
};
